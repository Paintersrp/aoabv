using Godot;
public partial class TimelineHud : VBoxContainer
{
    private Label? _statusLabel;
    private Label? _tickLabel;
    private Label? _highlightLabel;
    private Label? _chronicleLabel;

    public override void _Ready()
    {
        _statusLabel = GetNodeOrNull<Label>("StatusLabel");
        _tickLabel = GetNodeOrNull<Label>("TickLabel");
        _highlightLabel = GetNodeOrNull<Label>("HighlightLabel");
        _chronicleLabel = GetNodeOrNull<Label>("ChronicleLabel");
    }

    public void SetStatus(string text)
    {
        if (_statusLabel != null)
        {
            _statusLabel.Text = text;
        }
    }

    public void UpdateFrame(ulong tick, int highlightCount, string[]? chronicle)
    {
        if (_tickLabel != null)
        {
            _tickLabel.Text = $"Tick: {tick}";
        }

        if (_highlightLabel != null)
        {
            _highlightLabel.Text = $"Highlights: {highlightCount}";
        }

        if (_chronicleLabel != null)
        {
            var latestLine = chronicle != null && chronicle.Length > 0
                ? chronicle[^1]
                : "—";
            _chronicleLabel.Text = $"Chronicle: {latestLine}";
        }
    }
}
