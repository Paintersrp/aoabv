using Godot;
using System;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

public partial class WebSocketClient : Node
{
    private const string StreamUrl = "ws://127.0.0.1:7777/stream";
    private const int FrameBufferSize = 120;

    private readonly WebSocketPeer _peer = new();
    private readonly List<Frame> _frameBuffer = new();
    private readonly StringBuilder _incomingBuffer = new();
    private readonly JsonSerializerOptions _serializerOptions = new(JsonSerializerDefaults.General)
    {
        PropertyNameCaseInsensitive = false,
        AllowTrailingCommas = false,
        ReadCommentHandling = JsonCommentHandling.Disallow
    };

    private WebSocketPeer.State _lastState = WebSocketPeer.State.Closed;
    private MapRenderer? _mapRenderer;
    private TimelineHud? _timelineHud;

    public override void _Ready()
    {
        _mapRenderer = GetNodeOrNull<MapRenderer>("MapCanvas");
        _timelineHud = GetNodeOrNull<TimelineHud>("HudLayer/HudRoot/HudStack");

        _timelineHud?.SetStatus("Connecting…");
        var error = _peer.ConnectToUrl(StreamUrl);
        if (error != Error.Ok)
        {
            GD.PushError($"Failed to start WebSocket connection: {error}");
            _timelineHud?.SetStatus($"Connection error: {error}");
        }
    }

    public override void _Process(double delta)
    {
        _peer.Poll();
        var state = _peer.GetReadyState();
        if (state != _lastState)
        {
            UpdateStatus(state);
            _lastState = state;
        }

        if (state != WebSocketPeer.State.Open)
        {
            return;
        }

        while (_peer.GetAvailablePacketCount() > 0)
        {
            var packet = _peer.GetPacket();
            if (!_peer.WasTextPacket())
            {
                continue;
            }

            var text = Encoding.UTF8.GetString(packet);
            ConsumeText(text);
        }
    }

    public override void _ExitTree()
    {
        if (_peer.GetReadyState() == WebSocketPeer.State.Open || _peer.GetReadyState() == WebSocketPeer.State.Connecting)
        {
            _peer.Close();
        }
        _peer.Dispose();
    }

    private void ConsumeText(string text)
    {
        _incomingBuffer.Append(text);
        var bufferString = _incomingBuffer.ToString();
        var start = 0;
        int newline;
        while ((newline = bufferString.IndexOf('\n', start)) != -1)
        {
            var length = newline - start;
            if (length > 0)
            {
                var line = bufferString.Substring(start, length).Trim();
                if (!string.IsNullOrEmpty(line))
                {
                    ProcessFrameLine(line);
                }
            }
            start = newline + 1;
        }

        _incomingBuffer.Clear();
        if (start < bufferString.Length)
        {
            _incomingBuffer.Append(bufferString.AsSpan(start));
        }
    }

    private void ProcessFrameLine(string line)
    {
        try
        {
            var frame = JsonSerializer.Deserialize<Frame>(line, _serializerOptions);
            if (frame == null)
            {
                return;
            }

            _frameBuffer.Add(frame);
            if (_frameBuffer.Count > FrameBufferSize)
            {
                _frameBuffer.RemoveAt(0);
            }

            DispatchFrame(frame);
        }
        catch (Exception ex)
        {
            GD.PushError($"Failed to parse NDJSON frame: {ex.Message}");
        }
    }

    private void DispatchFrame(Frame frame)
    {
        var biomeDiff = new List<KeyValuePair<int, int>>();
        int maxRegionIndex = -1;

        if (frame.Diff?.Biome != null)
        {
            foreach (var kv in frame.Diff.Biome)
            {
                if (!TryParseRegionKey(kv.Key, out var regionIndex))
                {
                    continue;
                }

                biomeDiff.Add(new KeyValuePair<int, int>(regionIndex, kv.Value));
                if (regionIndex > maxRegionIndex)
                {
                    maxRegionIndex = regionIndex;
                }
            }
        }

        if (frame.Diff?.Hazards != null)
        {
            foreach (var hazard in frame.Diff.Hazards)
            {
                if (hazard.Region > maxRegionIndex)
                {
                    maxRegionIndex = hazard.Region;
                }
            }

            if (frame.Diff.Hazards.Count > 0)
            {
                var assumedTotal = frame.Diff.Hazards.Count;
                if (assumedTotal - 1 > maxRegionIndex)
                {
                    maxRegionIndex = assumedTotal - 1;
                }
            }
        }

        int? regionCountHint = maxRegionIndex >= 0 ? maxRegionIndex + 1 : null;
        _mapRenderer?.ApplyDiff(biomeDiff, regionCountHint);

        var highlightCount = frame.Highlights?.Length ?? 0;
        _timelineHud?.UpdateFrame(frame.Tick, highlightCount, frame.Chronicle);
    }

    private void UpdateStatus(WebSocketPeer.State state)
    {
        string statusText = state switch
        {
            WebSocketPeer.State.Connecting => "Connecting…",
            WebSocketPeer.State.Open => "Connected",
            WebSocketPeer.State.Closing => "Closing…",
            WebSocketPeer.State.Closed => "Disconnected",
            _ => "Unknown"
        };

        _timelineHud?.SetStatus($"Status: {statusText}");
    }

    private static bool TryParseRegionKey(string key, out int region)
    {
        region = -1;
        if (!key.StartsWith("r:", StringComparison.Ordinal))
        {
            return false;
        }

        return int.TryParse(key.AsSpan(2), out region);
    }

    private sealed class Frame
    {
        [JsonPropertyName("t")]
        public ulong Tick { get; set; }

        [JsonPropertyName("diff")]
        public FrameDiff? Diff { get; set; }

        [JsonPropertyName("highlights")]
        public FrameHighlight[]? Highlights { get; set; }

        [JsonPropertyName("chronicle")]
        public string[]? Chronicle { get; set; }

        [JsonPropertyName("era_end")]
        public bool EraEnd { get; set; }
    }

    private sealed class FrameDiff
    {
        [JsonPropertyName("biome")]
        public Dictionary<string, int>? Biome { get; set; }

        [JsonPropertyName("water")]
        public Dictionary<string, int>? Water { get; set; }

        [JsonPropertyName("soil")]
        public Dictionary<string, int>? Soil { get; set; }

        [JsonPropertyName("hazards")]
        public List<FrameHazard>? Hazards { get; set; }
    }

    private sealed class FrameHighlight
    {
        [JsonPropertyName("type")]
        public string Type { get; set; } = string.Empty;

        [JsonPropertyName("region")]
        public int Region { get; set; }

        [JsonPropertyName("info")]
        public JsonElement Info { get; set; }
    }

    private sealed class FrameHazard
    {
        [JsonPropertyName("region")]
        public int Region { get; set; }

        [JsonPropertyName("drought")]
        public int Drought { get; set; }

        [JsonPropertyName("flood")]
        public int Flood { get; set; }
    }
}
