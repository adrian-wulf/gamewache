# res://addons/gamewache/replay_player.gd
# Odtwarzacz deterministycznych powtórek wejść gracza klatka po klatce
extends Node

signal replay_frame_advanced(current_tick: int, total_ticks: int)
signal replay_finished()

var frames: Array[Dictionary] = []
var current_index: int = 0
var is_playing: bool = false
var playback_speed: float = 1.0

func load_replay_from_json(json_str: String) -> bool:
    var parsed = JSON.parse_string(json_str)
    if parsed is Array:
        frames.clear()
        for f in parsed:
            if f is Dictionary:
                frames.append(f)
        current_index = 0
        return true
    return false

func load_replay_from_base64(b64: String) -> bool:
    var json_str = Marshalls.base64_to_utf8(b64)
    return load_replay_from_json(json_str)

func play() -> void:
    is_playing = true

func pause() -> void:
    is_playing = false

func step_forward() -> void:
    if current_index < frames.size():
        apply_frame(frames[current_index])
        current_index += 1
        replay_frame_advanced.emit(current_index, frames.size())
    else:
        is_playing = false
        replay_finished.emit()

func step_backward() -> void:
    if current_index > 0:
        current_index -= 1
        apply_frame(frames[current_index])
        replay_frame_advanced.emit(current_index, frames.size())

func _physics_process(_delta: float) -> void:
    if is_playing:
        step_forward()

func apply_frame(frame: Dictionary) -> void:
    var mask: int = frame.get("mask", 0)

    # Symulacja akcji wejścia
    set_action_state("ui_up", (mask & (1 << 0)) != 0)
    set_action_state("ui_down", (mask & (1 << 1)) != 0)
    set_action_state("ui_left", (mask & (1 << 2)) != 0)
    set_action_state("ui_right", (mask & (1 << 3)) != 0)
    set_action_state("ui_accept", (mask & (1 << 4)) != 0)

func set_action_state(action_name: String, pressed: bool) -> void:
    if InputMap.has_action(action_name):
        if pressed and not Input.is_action_pressed(action_name):
            Input.action_press(action_name)
        elif not pressed and Input.is_action_pressed(action_name):
            Input.action_release(action_name)
