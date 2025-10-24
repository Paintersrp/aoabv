# Cause codes

| Code | Stage | Description |
| ---- | ----- | ----------- |
| `latitude_belt` | climate | Region biome assignment derived from its latitude band. Note explains the band label. |
| `orographic_lift` | climate | Prevailing-wind uplift over steep windward slopes; note reports `gradient_km` and `multiplier`. |
| `rain_shadow` | climate | Downwind moisture depletion from an upwind barrier; note reports `shadow_factor`. |
| `humidity_transport` | climate | Atmospheric moisture mixed from prior precipitation, stored insolation, and orographic flow adjustments. |
| `seasonal_shift` | climate | Seasonal sinusoid applied to regional temperature and precipitation (range -1.0..1.0). |
| `hadley_cell` | climate | Hadley circulation strength for low-latitude energy balance; note records `strength`. |
| `hadley_drift` | climate | Seasonal Hadley belt shift applied to effective latitude; note records `shift_deg`. |
| `energy_balance_adjustment` | climate (coupler) | Mean temperature baseline offset (tenths °C) queued by the atmosphere-cryosphere coupler; note records `mean_tenths`. |
| `monsoon_onset` | climate | Monsoon surge over humid low latitudes; note records `intensity`. |
| `storm_track_shift` | climate | Rolling precipitation spikes flagged from the extreme window; note records `anomaly_mm` and `window`. |
| `heat_extreme` | climate | Rolling heatwave index calculated from temperature maxima; note records `anomaly_tenths` and `window`. |
| `orogeny_belt` | geodynamics | Uplift event raised local terrain; note reports the signed metre delta. |
| `volcanic_aerosol_pulse` | geodynamics | Volcanic eruption injected aerosols; note records `region` and `optical_depth`. |
| `subsidence_deltas` | geodynamics | Subsidence lowered local terrain; note reports the signed metre delta. |
| `soil_fertility_low` | ecology | Soil value fell below the fertility floor (2_500). |
| `drought_flag` | ecology | Water level under 7_000 (scaled) after ecology adjustments. |
| `flood_flag` | ecology | Water level above 8_500 (scaled) after ecology adjustments. |
| `albedo_feedback` | cryosphere, coupler | Surface albedo updated from snow/ice coverage or the coupler feedback loop; note records `milli=<value>`. |
| `permafrost_thaw` | cryosphere | Active-layer depth increased due to seasonal thaw; note records `depth_cm`. |
| `glacier_mass_balance` | cryosphere | Degree-day accumulation versus melt; note records `balance_mm`. |
| `freshwater_pulse` | cryosphere | Freshwater discharge from ice melt; note records `tenths_mm`. |
| `snowmelt_surge` | cryosphere | Rapid snowpack melt entered regional waterways; note records `mm`. |
| `ice_mass_variation` | cryosphere | Regional glacier storage changed; note reports `delta_kt`. |
| `sea_level_contribution` | cryosphere | Meltwater raised global mean sea level; note reports `mm`. |
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
