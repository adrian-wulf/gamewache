// GameWache Unity SDK (C#)
using System;
using System.Collections;
using System.Collections.Generic;
using System.Text;
using UnityEngine;
using UnityEngine.Networking;

namespace GameWache.Runtime
{
    [DefaultExecutionOrder(-1000)]
    public class GameWacheClient : MonoBehaviour
    {
        public static GameWacheClient Instance { get; private set; }

        [Header("Konfiguracja GameWache")]
        public string endpoint = "http://127.0.0.1:8090";
        public string projectId = "unity-game";
        public bool enableCrashHandler = true;
        public bool enableBlackboxReplay = true;

        private uint currentTick = 0;
        private readonly List<InputFrameRecord> replayBuffer = new List<InputFrameRecord>(1800);
        private readonly List<BreadcrumbRecord> breadcrumbs = new List<BreadcrumbRecord>(100);

        [Serializable]
        public struct InputFrameRecord
        {
            public uint tick;
            public uint buttons;
            public float lx;
            public float ly;
        }

        [Serializable]
        public struct BreadcrumbRecord
        {
            public long timestamp_ms;
            public uint engine_tick;
            public string category;
            public string message;
        }

        private void Awake()
        {
            if (Instance != null && Instance != this)
            {
                Destroy(gameObject);
                return;
            }
            Instance = this;
            DontDestroyOnLoad(gameObject);

            if (enableCrashHandler)
            {
                Application.logMessageReceivedThreaded += HandleLog;
                AppDomain.CurrentDomain.UnhandledException += HandleUnhandledException;
            }
        }

        private void FixedUpdate()
        {
            currentTick++;
            if (enableBlackboxReplay)
            {
                RecordInput();
            }
        }

        private void RecordInput()
        {
            uint mask = 0;
            if (Input.GetKey(KeyCode.W) || Input.GetKey(KeyCode.UpArrow)) mask |= (1 << 0);
            if (Input.GetKey(KeyCode.S) || Input.GetKey(KeyCode.DownArrow)) mask |= (1 << 1);
            if (Input.GetKey(KeyCode.A) || Input.GetKey(KeyCode.LeftArrow)) mask |= (1 << 2);
            if (Input.GetKey(KeyCode.D) || Input.GetKey(KeyCode.RightArrow)) mask |= (1 << 3);
            if (Input.GetKey(KeyCode.Space)) mask |= (1 << 4);
            if (Input.GetMouseButton(0)) mask |= (1 << 5);

            var frame = new InputFrameRecord
            {
                tick = currentTick,
                buttons = mask,
                lx = Input.GetAxisRaw("Horizontal"),
                ly = Input.GetAxisRaw("Vertical")
            };

            if (replayBuffer.Count >= 1800)
            {
                replayBuffer.RemoveAt(0);
            }
            replayBuffer.Add(frame);
        }

        public void AddBreadcrumb(string category, string message)
        {
            var b = new BreadcrumbRecord
            {
                timestamp_ms = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                engine_tick = currentTick,
                category = category,
                message = message
            };
            if (breadcrumbs.Count >= 100) breadcrumbs.RemoveAt(0);
            breadcrumbs.Add(b);
        }

        private void HandleLog(string logString, string stackTrace, LogType type)
        {
            if (type == LogType.Exception || type == LogType.Error)
            {
                ReportCrash(type.ToString(), logString, stackTrace);
            }
        }

        private void HandleUnhandledException(object sender, UnhandledExceptionEventArgs e)
        {
            var ex = e.ExceptionObject as Exception;
            if (ex != null)
            {
                ReportCrash("UnhandledException", ex.Message, ex.StackTrace);
            }
        }

        public void ReportCrash(string exceptionType, string message, string stackTrace)
        {
            StartCoroutine(SendCrashRoutine(exceptionType, message, stackTrace));
        }

        private IEnumerator SendCrashRoutine(string exceptionType, string message, string stackTrace)
        {
            var payload = new CrashPayload
            {
                project_id = projectId,
                exception_type = exceptionType,
                message = message,
                severity = "fatal",
                platform = Application.platform.ToString().ToLower(),
                engine = "unity",
                engine_version = Application.unityVersion,
                engine_tick = currentTick
            };

            string json = JsonUtility.ToJson(payload);
            using (var req = new UnityWebRequest(endpoint + "/api/v1/crashes", "POST"))
            {
                byte[] bodyRaw = Encoding.UTF8.GetBytes(json);
                req.uploadHandler = new UploadHandlerRaw(bodyRaw);
                req.downloadHandler = new DownloadHandlerBuffer();
                req.SetRequestHeader("Content-Type", "application/json");
                yield return req.SendWebRequest();
            }
        }

        [Serializable]
        private class CrashPayload
        {
            public string project_id;
            public string exception_type;
            public string message;
            public string severity;
            public string platform;
            public string engine;
            public string engine_version;
            public uint engine_tick;
        }
    }
}
