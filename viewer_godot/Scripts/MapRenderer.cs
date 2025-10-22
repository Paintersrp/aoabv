using Godot;
using System;
using System.Collections.Generic;

public partial class MapRenderer : Node2D
{
    private const int TileSize = 8;

    private static readonly Color[] BiomePalette =
    {
        new Color("#2f4858"),
        new Color("#33658a"),
        new Color("#55dde0"),
        new Color("#f6ae2d")
    };

    private static readonly Color DefaultColor = new Color("#1b1b2f");

    private byte[] _biomes = Array.Empty<byte>();
    private Image? _image;
    private ImageTexture? _texture;
    private int _width;
    private int _height;
    private bool _imageDirty;

    public override void _Ready()
    {
        SetProcess(false);
    }

    public override void _Draw()
    {
        if (_texture == null)
        {
            return;
        }

        var size = new Vector2(_width, _height) * TileSize;
        DrawTextureRect(_texture, new Rect2(Vector2.Zero, size), false);
    }

    public void ApplyDiff(IEnumerable<KeyValuePair<int, int>>? biomeDiff, int? regionCountHint)
    {
        if (regionCountHint.HasValue && regionCountHint.Value > 0)
        {
            EnsureCapacity(regionCountHint.Value);
        }

        if (biomeDiff == null)
        {
            return;
        }

        foreach (var change in biomeDiff)
        {
            if (change.Key < 0)
            {
                continue;
            }

            EnsureCapacity(change.Key + 1);
            UpdateBiome(change.Key, change.Value);
        }

        CommitIfDirty();
    }

    private void EnsureCapacity(int requiredRegions)
    {
        if (requiredRegions <= 0)
        {
            return;
        }

        if (_biomes.Length >= requiredRegions && _width * _height >= requiredRegions)
        {
            return;
        }

        var newCount = Math.Max(requiredRegions, _biomes.Length);
        if (newCount == 0)
        {
            newCount = requiredRegions;
        }

        var oldLength = _biomes.Length;
        Array.Resize(ref _biomes, newCount);
        if (newCount > oldLength)
        {
            Array.Fill(_biomes, (byte)0, oldLength, newCount - oldLength);
        }

        RebuildSurface(newCount);
    }

    private void UpdateBiome(int regionIndex, int biomeCode)
    {
        if (_biomes.Length == 0)
        {
            return;
        }

        var stored = (byte)Math.Clamp(biomeCode, 0, 255);
        if (_biomes[regionIndex] == stored)
        {
            return;
        }

        _biomes[regionIndex] = stored;
        WriteCell(regionIndex, stored);
    }

    private void WriteCell(int regionIndex, byte biomeCode)
    {
        if (_image == null || _width == 0)
        {
            return;
        }

        var color = biomeCode < BiomePalette.Length ? BiomePalette[biomeCode] : DefaultColor;
        var x = regionIndex % _width;
        var y = regionIndex / _width;
        var rect = new Rect2I(x * TileSize, y * TileSize, TileSize, TileSize);
        _image!.FillRect(rect, color);
        _imageDirty = true;
    }

    private void CommitIfDirty()
    {
        if (!_imageDirty || _image == null)
        {
            return;
        }

        if (_texture == null)
        {
            _texture = ImageTexture.CreateFromImage(_image);
        }
        else
        {
            _texture.Update(_image);
        }

        _imageDirty = false;
        QueueRedraw();
    }

    private void RebuildSurface(int regionCount)
    {
        if (regionCount <= 0)
        {
            _width = 0;
            _height = 0;
            _image = null;
            _texture = null;
            return;
        }

        (_width, _height) = DeriveDimensions(regionCount);
        var pixelWidth = Math.Max(_width * TileSize, TileSize);
        var pixelHeight = Math.Max(_height * TileSize, TileSize);
        _image = Image.Create(pixelWidth, pixelHeight, false, Image.Format.Rgba8);
        _image.Fill(DefaultColor);

        for (var index = 0; index < _biomes.Length; index++)
        {
            WriteCell(index, _biomes[index]);
        }

        _texture = ImageTexture.CreateFromImage(_image);
        _imageDirty = false;
        QueueRedraw();
    }

    private static (int width, int height) DeriveDimensions(int regionCount)
    {
        var bestWidth = regionCount;
        var bestHeight = 1;
        var bestDiff = regionCount - 1;

        var maxFactor = (int)Math.Sqrt(regionCount);
        for (var candidate = 1; candidate <= maxFactor; candidate++)
        {
            if (regionCount % candidate != 0)
            {
                continue;
            }

            var other = regionCount / candidate;
            var diff = Math.Abs(other - candidate);
            if (diff < bestDiff)
            {
                bestDiff = diff;
                bestWidth = Math.Max(other, candidate);
                bestHeight = Math.Min(other, candidate);
            }
        }

        if (bestWidth < bestHeight)
        {
            (bestWidth, bestHeight) = (bestHeight, bestWidth);
        }

        return (bestWidth, bestHeight);
    }
}
