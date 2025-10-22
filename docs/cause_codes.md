# Cause codes

| Code | Stage | Description |
| ---- | ----- | ----------- |
| `latitude_belt` | climate | Region biome assignment derived from its latitude band. Note explains the band label. |
| `seasonality_variance` | climate | Seasonal variance applied to the region's moisture budget (range -1.0..1.0). |
| `soil_fertility_low` | ecology | Soil value fell below the fertility floor (2_500). |
| `drought_flag` | ecology | Water level under 7_000 (scaled) after ecology adjustments. |
| `flood_flag` | ecology | Water level above 8_500 (scaled) after ecology adjustments. |
| `era_end` | meta | Reserved for future milestones (unused in v0.0). |
| `stagnation_warning` | meta | Reserved hook for growth stalls (unused in v0.0). |
| `collapse_warning` | meta | Reserved hook for catastrophic collapse (unused in v0.0). |
