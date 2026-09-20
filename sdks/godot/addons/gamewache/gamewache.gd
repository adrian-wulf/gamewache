# res://addons/gamewache/gamewache.gd
# Autonomiczny klient GameWache dla silnika Godot 4.x
extends Node

signal live_tweak_applied(param_path: String, new_value: Variant)
signal command_executed(cmd: String)

var endpoint: String = "http://127.0.0.1:8090"
var ws_endpoint: String = "ws://127.0.0.1:8090/ws/live_tuning"
var project_id: String = "panzer-hookandshoot"
var engine_version: String = "Godot 4.x"

var enable_crash_handler: bool = true
var enable_blackbox_replay: bool = true
var enable_live_tweaker: bool = true
var enable_shader_tracker: bool = true
var enable_f11_reporter: bool = true

# Breadcrumbs Ring Buffer
const MAX_BREADCRUMBS: int = 200
var breadcrumbs: Array[Dictionary] = []

# Blackbox Replay Ring Buffer (60 FPS * 30 sekund = 1800 klatek)
const REPLAY_CAPACITY: int = 1800
var replay_buffer: Array[Dictionary] = []
var replay_index: int = 0
var current_tick: int = 0
var initial_rng_seed: int = 0

# Live Tweaker WebSocket
var ws_peer: WebSocketPeer
var ws_connected: bool = false
var registered_tweaks: Dictionary = {}
var registered_commands: Dictionary = {}

# Bug Reporter Node
var reporter_dialog: CanvasLayer = null

func _ready() -> void:
    process_mode = Node.PROCESS_MODE_ALWAYS
    initial_rng_seed = randi()
    
    if enable_live_tweaker:
        init_websocket()
        
    add_breadcrumb("System", "GameWache SDK zainicjalizowany w Godot 4", {"seed": initial_rng_seed})

func initialize(config: Dictionary) -> void:
    if config.has("endpoint"): endpoint = config["endpoint"]
    if config.has("project_id"): project_id = config["project_id"]
    if config.has("ws_endpoint"): ws_endpoint = config["ws_endpoint"]
    if config.has("enable_blackbox_replay"): enable_blackbox_replay = config["enable_blackbox_replay"]
    if config.has("enable_live_tweaker"): enable_live_tweaker = config["enable_live_tweaker"]
    if config.has("enable_f11_reporter"): enable_f11_reporter = config["enable_f11_reporter"]

func _physics_process(_delta: float) -> void:
    current_tick += 1
    
    if enable_blackbox_replay:
        record_input_frame()

func _process(delta: float) -> void:
    if enable_live_tweaker:
        poll_websocket()
        
    # Shader Stutter / Hitch Tracker (>50ms frame spike)
    if enable_shader_tracker and delta > 0.050:
        record_hitch(delta)

func _unhandled_input(event: InputEvent) -> void:
    if enable_f11_reporter and event is InputEventKey:
        if event.pressed and event.keycode == KEY_F11:
            toggle_bug_reporter()

# --- REPLAY BLACKBOX ENGINE ---

func record_input_frame() -> void:
    var digital_mask: int = 0
    if Input.is_action_pressed("ui_up"): digital_mask |= (1 << 0)
    if Input.is_action_pressed("ui_down"): digital_mask |= (1 << 1)
    if Input.is_action_pressed("ui_left"): digital_mask |= (1 << 2)
    if Input.is_action_pressed("ui_right"): digital_mask |= (1 << 3)
    if Input.is_action_pressed("ui_accept"): digital_mask |= (1 << 4)
    if Input.is_mouse_button_pressed(MOUSE_BUTTON_LEFT): digital_mask |= (1 << 5)
    if Input.is_mouse_button_pressed(MOUSE_BUTTON_RIGHT): digital_mask |= (1 << 6)

    var move_vec: Vector2 = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down")
    var mouse_pos: Vector2 = get_viewport().get_mouse_position() if get_viewport() else Vector2.ZERO

    var frame: Dictionary = {
        "tick": current_tick,
        "mask": digital_mask,
        "lx": move_vec.x,
        "ly": move_vec.y,
        "mx": mouse_pos.x,
        "my": mouse_pos.y
    }

    if replay_buffer.size() < REPLAY_CAPACITY:
        replay_buffer.append(frame)
    else:
        replay_buffer[replay_index] = frame

    replay_index = (replay_index + 1) % REPLAY_CAPACITY

func get_ordered_replay() -> Array[Dictionary]:
    var ordered: Array[Dictionary] = []
    if replay_buffer.size() < REPLAY_CAPACITY:
        return replay_buffer.duplicate()
    
    for i in range(replay_index, REPLAY_CAPACITY):
        ordered.append(replay_buffer[i])
    for i in range(0, replay_index):
        ordered.append(replay_buffer[i])
    return ordered

# --- BREADCRUMBS & TELEMETRIA ---

func add_breadcrumb(category: String, message: String, metadata: Dictionary = {}, pos: Variant = null) -> void:
    var player_pos: Variant = null
    if pos is Vector2:
        player_pos = [pos.x, pos.y, 0.0]
    elif pos is Vector3:
        player_pos = [pos.x, pos.y, pos.z]

    var current_scene: String = get_tree().current_scene.scene_file_path if get_tree().current_scene else ""

    var b: Dictionary = {
        "timestamp_ms": Time.get_ticks_msec(),
        "engine_tick": current_tick,
        "category": category,
        "message": message,
        "level_name": current_scene,
        "player_pos": player_pos,
        "metadata": metadata
    }

    breadcrumbs.append(b)
    if breadcrumbs.size() > MAX_BREADCRUMBS:
        breadcrumbs.pop_front()

func record_hitch(delta: float) -> void:
    var hitch_ms: float = delta * 1000.0
    var renderer_info: String = RenderingServer.get_video_adapter_name() + " (" + RenderingServer.get_video_adapter_api_version() + ")"
    var current_scene: String = get_tree().current_scene.scene_file_path if get_tree().current_scene else "unknown"

    var payload: Dictionary = {
        "project_id": project_id,
        "shader_path": "RenderingServer/HitchSpike",
        "hitch_duration_ms": hitch_ms,
        "gpu_renderer": renderer_info,
        "render_pass": "Forward+",
        "level_name": current_scene
    }

    send_http_json("/api/v1/shaders", payload)

# --- CRASH & ERROR INGESTION ---

func report_crash(exception_type: String, message: String, stack: Array = [], pos: Variant = null) -> void:
    var player_pos: Variant = null
    if pos is Vector2:
        player_pos = [pos.x, pos.y, 0.0]
    elif pos is Vector3:
        player_pos = [pos.x, pos.y, pos.z]

    var current_scene: String = get_tree().current_scene.scene_file_path if get_tree().current_scene else ""

    # Zapisz powtórkę w tle
    var replay_id: String = "rec_" + str(Time.get_unix_time_from_system()).replace(".", "")
    dump_replay(replay_id, current_scene)

    var report: Dictionary = {
        "project_id": project_id,
        "exception_type": exception_type,
        "message": message,
        "stacktrace": stack,
        "source_file": stack[0]["filename"] if stack.size() > 0 and stack[0].has("filename") else "",
        "line_no": stack[0]["lineno"] if stack.size() > 0 and stack[0].has("lineno") else 0,
        "severity": "fatal",
        "platform": OS.get_name().to_lower(),
        "engine": "godot",
        "engine_version": Engine.get_version_info()["string"],
        "engine_tick": current_tick,
        "scene_name": current_scene,
        "spatial_pos": player_pos,
        "breadcrumbs": breadcrumbs,
        "replay_id": replay_id,
        "hardware_info": {
            "gpu": RenderingServer.get_video_adapter_name(),
            "os": OS.get_distribution_name(),
            "screen": DisplayServer.screen_get_size()
        }
    }

    send_http_json("/api/v1/crashes", report)

func dump_replay(replay_id: String, scene_name: String) -> void:
    var frames: Array[Dictionary] = get_ordered_replay()
    var replay_data: Dictionary = {
        "id": replay_id,
        "project_id": project_id,
        "duration_secs": float(frames.size()) / 60.0,
        "tick_count": frames.size(),
        "scene_name": scene_name,
        "rng_seed": initial_rng_seed,
        "initial_snapshot": {},
        "input_stream_base64": Marshalls.utf8_to_base64(JSON.stringify(frames))
    }
    send_http_json("/api/v1/replays", replay_data)

# --- LIVE WEBSOCKET TWEAKER ---

func init_websocket() -> void:
    ws_peer = WebSocketPeer.new()
    ws_peer.connect_to_url(ws_endpoint)

func poll_websocket() -> void:
    if ws_peer == null:
        return
    ws_peer.poll()
    var state = ws_peer.get_ready_state()

    if state == WebSocketPeer.STATE_OPEN:
        if not ws_connected:
            ws_connected = true
            # Wyślij wszystkie zarejestrowane zmienne do serwera
            for path in registered_tweaks:
                var item = registered_tweaks[path]
                send_ws_register(path, item["target"].get(item["prop"]), item["meta"])

        while ws_peer.get_available_packet_count() > 0:
            var pkt = ws_peer.get_packet().get_string_from_utf8()
            handle_ws_packet(pkt)
    elif state == WebSocketPeer.STATE_CLOSED:
        ws_connected = false

func register_tweak(path: String, target: Object, prop_name: String, meta: Dictionary = {}) -> void:
    registered_tweaks[path] = {
        "target": target,
        "prop": prop_name,
        "meta": meta
    }
    if ws_connected and target != null:
        send_ws_register(path, target.get(prop_name), meta)

func register_command(cmd_name: String, callable: Callable) -> void:
    registered_commands[cmd_name] = callable

func send_ws_register(path: String, val: Variant, meta: Dictionary) -> void:
    var msg: Dictionary = {
        "msg_type": "register",
        "param_path": path,
        "value": val,
        "meta": meta
    }
    if ws_peer and ws_peer.get_ready_state() == WebSocketPeer.STATE_OPEN:
        ws_peer.send_text(JSON.stringify(msg))

func handle_ws_packet(pkt: String) -> void:
    var json_res = JSON.parse_string(pkt)
    if not (json_res is Dictionary):
        return

    var msg_type: String = json_res.get("msg_type", "")
    if msg_type == "tweak":
        var path: String = json_res.get("param_path", "")
        var new_val: Variant = json_res.get("value", null)
        if registered_tweaks.has(path):
            var item = registered_tweaks[path]
            var obj: Object = item["target"]
            if is_instance_valid(obj):
                obj.set(item["prop"], new_val)
                live_tweak_applied.emit(path, new_val)
    elif msg_type == "command":
        var cmd: String = json_res.get("cmd", "")
        var parts = cmd.split(" ")
        if parts.size() > 0:
            var cname = parts[0]
            if registered_commands.has(cname):
                var cb: Callable = registered_commands[cname]
                if cb.is_valid():
                    cb.call()
                    command_executed.emit(cmd)

# --- F11 BUG REPORTER ---

func toggle_bug_reporter() -> void:
    if reporter_dialog != null and is_instance_valid(reporter_dialog):
        reporter_dialog.queue_free()
        reporter_dialog = null
        return

    # Dynamiczne wygenerowanie nakładki zgłoszeniowej
    reporter_dialog = CanvasLayer.new()
    reporter_dialog.layer = 120

    var panel = PanelContainer.new()
    panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
    panel.custom_minimum_size = Vector2(480, 320)

    var vbox = VBoxContainer.new()
    vbox.add_theme_constant_override("separation", 8)

    var title = Label.new()
    title.text = "🎮 GameWache Playtest Reporter (F11)"
    title.add_theme_font_size_override("font_size", 16)
    vbox.add_child(title)

    var desc = TextEdit.new()
    desc.placeholder_text = "Opisz krótko błąd lub spostrzeżenie..."
    desc.custom_minimum_size = Vector2(460, 100)
    vbox.add_child(desc)

    var hbox_btns = HBoxContainer.new()
    var btn_cancel = Button.new()
    btn_cancel.text = "Anuluj (Esc)"
    btn_cancel.pressed.connect(func(): reporter_dialog.queue_free(); reporter_dialog = null)
    hbox_btns.add_child(btn_cancel)

    var btn_send = Button.new()
    btn_send.text = "🚀 Wyślij Zgłoszenie z Powtórką"
    btn_send.pressed.connect(func(): submit_playtest_report(desc.text))
    hbox_btns.add_child(btn_send)

    vbox.add_child(hbox_btns)
    panel.add_child(vbox)
    reporter_dialog.add_child(panel)
    get_tree().root.add_child(reporter_dialog)

func submit_playtest_report(comment: String) -> void:
    var current_scene: String = get_tree().current_scene.scene_file_path if get_tree().current_scene else ""
    var replay_id: String = "rec_" + str(Time.get_unix_time_from_system()).replace(".", "")
    dump_replay(replay_id, current_scene)

    # Zrzut ekranu (WebP)
    var img = get_viewport().get_texture().get_image()
    var base64_img: String = ""
    if img:
        var buf = img.save_webp_to_buffer()
        base64_img = Marshalls.raw_to_base64(buf)

    var payload: Dictionary = {
        "id": "fb_" + str(Time.get_unix_time_from_system()).replace(".", ""),
        "project_id": project_id,
        "user_comment": comment,
        "category": "bug",
        "screenshot_base64": base64_img,
        "replay_id": replay_id,
        "scene_name": current_scene,
        "user_system_info": {
            "gpu": RenderingServer.get_video_adapter_name(),
            "os": OS.get_name(),
            "fps": Engine.get_frames_per_second()
        }
    }

    send_http_json("/api/v1/feedback", payload)

    if reporter_dialog:
        reporter_dialog.queue_free()
        reporter_dialog = null

# --- HTTP ASYNC HELPER ---

func send_http_json(path: String, data: Dictionary) -> void:
    var http = HTTPRequest.new()
    add_child(http)
    http.request_completed.connect(func(_res, _code, _hdr, _body): http.queue_free())

    var headers: PackedStringArray = ["Content-Type: application/json"]
    var json_str = JSON.stringify(data)
    http.request(endpoint + path, headers, HTTPClient.METHOD_POST, json_str)
