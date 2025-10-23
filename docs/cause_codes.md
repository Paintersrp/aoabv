# Cause codes

| Code | Stage | Description |
| ---- | ----- | ----------- |
| `latitude_belt` | climate | Region biome assignment derived from its latitude band. Note explains the band label. |
| `orographic_lift` | climate | Elevated region relative to neighbours; note reports `lift_km=<value>` in kilometers. |
| `seasonality_variance` | climate | Seasonal variance applied to the region's moisture budget (range -1.0..1.0). |
| `orogeny_belt` | geodynamics | Uplift event raised local terrain; note reports the signed metre delta. |
| `volcanic_aerosol_pulse` | geodynamics | Volcanic eruption injected aerosols; note records `region` and `optical_depth`. |
| `subsidence_deltas` | geodynamics | Subsidence lowered local terrain; note reports the signed metre delta. |
| `soil_fertility_low` | ecology | Soil value fell below the fertility floor (2_500). |
| `drought_flag` | ecology | Water level under 7_000 (scaled) after ecology adjustments. |
| `flood_flag` | ecology | Water level above 8_500 (scaled) after ecology adjustments. |
| `era_end` | meta | Reserved for future milestones (unused in v0.0). |
| `stagnation_warning` | meta | Reserved hook for growth stalls (unused in v0.0). |
| `collapse_warning` | meta | Reserved hook for catastrophic collapse (unused in v0.0). |
| `cme_event` | astronomy | Coronal mass ejection injects a transient irradiance spike; note records `severity`. |
| `insolation_gradient` | astronomy | Latitude-driven insolation contrast; note records `delta_wm2`. |
| `obliquity_shift` | astronomy | Planetary axial tilt adjustment; note records `delta_deg`. |
| `precession_phase` | astronomy | Precession cycle update; note records `phase_deg`. |
| `solar_cycle_peak` | astronomy | Solar cycle peak influences irradiance; note records `cycle_index`. |
| `tide_neap` | astronomy | Neap tide envelope reduces tidal range; note records `phase`. |
| `tide_spring` | astronomy | Spring tide envelope amplifies tidal range; note records `phase`. |
