Planet Simulation — Master Plan (Long-Term)

principles
• Deterministic & explainable: every material change stores a compact cause[].
• Multi-rate: very slow (tectonics) ↔ very fast (storms) using staged sub-steps aggregated to a 5-year narrative tick.
• Modular: each system ships with an MVP that’s “good enough,” and a clear path to richer models.

0) System DAG (dataflow overview)

ASTRONOMY (star–planet–moon, magnetosphere, radiation)
  → insolation, spectrum, tide potential, space weather
GEODYNAMICS (interior, plates, volcanism, isostasy, magnetodynamo)
  → DEM, uplift/subsidence, aerosols, gases, magnetic field
ATMOSPHERE (energy balance, circulation, moisture, chemistry)
  ↔ OCEAN (SST, currents, sea level, salinity)
  ↔ CRYOSPHERE (ice sheets, glaciers, sea ice)
  → temp, precip, winds, storms, humidity
HYDROSPHERE (rivers, groundwater, wetlands, snowmelt)
  → discharge, flood/drought indices, aquifer state
BIOGEOCHEM (C/N/P/S/Si cycles, soils)
  ↔ BIOSPHERE (biomes, NPP, food webs, fire, disease ecology)
TERRAIN/EROSION (fluvial, coastal, aeolian, mass wasting)
EXTREME EVENTS (impacts, CMEs, mega-volcano, tsunamis)
HAZARD COMPOSER (compound events & return periods)

Narrative tick = 5 years. Each subsystem advances at its own cadence and accumulates/peaks are downsampled into the frame.

⸻

1) Astronomy & Space Environment (not astrology 🙂)

State: star_class, luminosity, spectrum, orbital_eccentricity, semi_major_axis, obliquity, precession, nutation, rotation_rate, moon_mass, moon_distance, tidal_Q, lunar_recession, solar_cycle_idx, flare_rate, CME_rate, cosmic_ray_flux, magnetosphere_strength.

Outputs: insolation(lat, day), seasonality_index, tide_envelope(lat,lon, phase), UV/IR fractions, space_weather_events.

MVP
	•	Analytic daily/seasonal insolation by latitude & obliquity.
	•	Tide envelope: spring/neap sine with per-basin amplification constant.
	•	Solar cycle scalar for insolation variance.

Advanced
	•	Milankovitch cycles (eccentricity, obliquity, precession) over kyr.
	•	Harmonic tide constituents (M2, S2, K1, O1) + 18.6-year nodal cycle; local resonance & shelf effects.
	•	Space weather: CME/flare events; magnetospheric coupling; atmospheric escape on long timescales.

Algorithms: Berger insolation formula; simple harmonic tides; Poisson flaring; magnetosphere as shielding factor.

Cause codes: insolation_gradient, obliquity_shift, precession_phase, tide_spring, tide_neap, solar_cycle_peak, cme_event.

Validation: Hadley edge ~30° for Earthlike; spring/neap ~14–15-day cycle (aggregates to flags at 5-yr scale).

⸻

2) Interior & Geodynamics

State: heat_budget(radioactive, primordial), mantle_convection_idx, plate_map, boundary_types, plume_hotspots, uplift_rate, subsidence_rate, degassing (CO2, SO2), aerosol_optical_depth, core_dynamo_strength.

Outputs: DEM (elevation/bathymetry), volcanic_pulses, earthquake_rate, mag_field_intensity.

MVP
	•	Static continents + procedural orogeny belts; rare volcanic pulses adding AOD for N years (cooling); simple uplift/subsidence on coasts.

Advanced
	•	Plate motion over Myr (simple kinematic model); ridge spreading + subduction arcs; flood basalts; secular variation in magnetic field.

Algorithms: cellular plate tectonics or kinematic plates; AOD cooling impulse response; flexural isostasy.

Cause codes: orogeny_belt, volcanic_aerosol_pulse, subsidence_deltas, degassing.

Validation: orographic rain shadows; transient cooling after large eruptions.

⸻

3) Atmosphere (Energy, Circulation, Moisture, Chemistry)

State: TOA_radiation, greenhouse_mix (CO2, CH4, H2O, O3), lapse_rate, jets, trade_winds, monsoon_switch, humidity fields, aerosol_load.

Outputs: temp(lat,lon,season), precip, humidity, winds, storm_events, drought_index.

MVP
	•	1–2D energy-balance + prescribed circulation cells (Hadley/Ferrel/Polar) from insolation; orographic uplift; precip from moisture_flux × lift; simple monsoon switch via land–sea thermal contrast.

Advanced
	•	ENSO-like oscillation toggling convection; Rossby wave–driven jet shifts; cyclone genesis probability from SST + shear; tropospheric chemistry (ozone, SO2).

Algorithms: EBM; moist adiabat; bucket hydrology for soil moisture; cyclone genesis index (e.g., Emanuel-like proxy).

Cause codes: hadley_cell, orographic_lift, monsoon_onset, storm_track_shift, enso_phase, aerosol_radiative_forcing.

Validation: ITCZ near thermal equator; realistic monsoon seasonality; drought clusters under persistent anomalies.

⸻

4) Ocean (SST, Circulation, Salinity, Sea Level)

State: SST, SSS, mixed_layer_depth, gyre_index, overturning_strength, sea_level (steric + eustatic + dynamic).

Outputs: surface currents, SST/SSS, sea level anomaly, upwelling zones.

MVP
	•	Wind-driven gyres from zonal winds; bulk mixed-layer energy balance (SST relaxes to insolation with lag); global mean sea level from ocean heat + ice mass.

Advanced
	•	Thermohaline overturning proxy; Kelvin/Rossby basin modes (very light); storm surge setup parameterization (for hazards coupling).

Algorithms: slab ocean model; linear gyre streamfunction; steric sea level from heat content.

Cause codes: wind_gyre_coupling, thermohaline_shift, sea_level_thermal_expansion, storm_surge_setup.

Validation: western boundary current warming; sea level rise with heat content.

⸻

5) Cryosphere (Ice Sheets, Glaciers, Permafrost, Sea Ice)

State: ice_sheet_mass, glacier_extent, permafrost_active_layer, sea_ice_fraction, albedo_map.

Outputs: albedo, freshwater_flux, sea-level contribution.

MVP
	•	Degree-day mass balance: accumulation = cold-season precip; melt = f(temp); albedo feedback.

Advanced
	•	Shallow-ice dynamics; calving; iceberg drift; permafrost carbon release.

Algorithms: degree-day; shallow-ice approximation; permafrost thermal model (1D).

Cause codes: albedo_feedback, glacier_mass_balance, freshwater_pulse, permafrost_thaw.

Validation: polar amplification; albedo–temperature coupling.

⸻

6) Hydrosphere (Rivers, Groundwater, Wetlands, Snowmelt)

State: river_network (from DEM), soil_moisture, snowpack, aquifer_storage, wetland_extent.

Outputs: discharge(edge,t), flood_flags(region), drought_index, baseflow, delta_growth.

MVP
	•	Flow routing (D8/D∞); linear reservoir runoff; degree-day snowmelt; flood flag if (discharge > Qcrit × (1 + tide_envelope + surge)) ⇒ compound flooding.

Advanced
	•	Muskingum-Cunge routing; groundwater recharge/depletion; floodplain storage; seiche effects in large lakes; reservoir rules (later).

Algorithms: HBV/GR4J-like rainfall-runoff; Muskingum routing; TOPMODEL-style saturation.

Cause codes: rain_runoff, snowmelt_surge, tide_compound_flood, soil_infiltration_limit, seiche_event.

Validation: seasonal hydrographs; coastal floods co-occurring with spring tide + storms.

⸻

7) Terrain, Erosion, and Soils

State: erodibility, sediment_supply, wave_exposure, soil_depth, regolith.

Outputs: elevation_change, soil_fertility, coastal_retreat, landslide_flags.

MVP
	•	Stream-power erosion E ∝ Q^m * slope^n; simple coastal retreat under surge + wave exposure; soil formation vs erosion budget.

Advanced
	•	Sediment routing to deltas; avulsions; aeolian dunes; karst dissolution; landslide probability from slope + rain.

Cause codes: fluvial_erosion, coastal_retreat, delta_progradation, mass_wasting_event.

Validation: rivers incise steep/high-Q basins; deltas prograde with high sediment/low sea level rise.

⸻

8) Biogeochemistry (C/N/P/S/Si) & Water Quality

State: multi-box carbon (atmos, surface ocean, deep ocean, land biomass, soils), nitrogen availability, phosphorus, methane wetlands.

Outputs: NPP (land/ocean), soil_fertility, radiative_forcing(ΔCO2), hypoxia_flags (coastal dead zones).

MVP
	•	4-box carbon; NPP = f(temp, moisture, nutrients); soil carbon turnover; simple hypoxia trigger = high nutrients + low mixing.

Advanced
	•	Full N and P cycles; iron fertilization; DOC/POC; water quality (DO, algae blooms).

Cause codes: photosynthesis_uptake, respiration_release, erosion_loss, volcanic_outgassing, eutrophication.

Validation: CO₂ lowers with greening/cooling; hypoxia under high N + stratified SST.

⸻

9) Biosphere & Ecology (pre-sentience)

State: biome_class, biodiversity_index, trophic_structure, fire_regime, disease_ecology.

Outputs: carrying_capacity(region), disturbance_events (fire/pest), invasion_flags.

MVP
	•	Köppen-style biomes from temp/precip + soil; NPP→carrying capacity; basic fire weather.

Advanced
	•	Succession; predator–prey oscillations; invasive species; zoonotic disease cycles (coupled to climate).

Cause codes: biome_shift_climate, fire_weather, drought_dieoff, outbreak_conditions.

Validation: belts track climate; fire peaks in hot, dry, windy regimes.

⸻

10) Extreme Events & Space Hazards

State: impactor_flux, size_dist, tectonic_tsuanmi_risk, CME_rate.

Outputs: impact_events (AOD + tsunamis), geomagnetic_storms.

MVP
	•	Poisson impacts; aerosol “winter” impulse; tsunami flags on ocean impacts.

Advanced
	•	Regional tsunami propagation proxy; magnetosphere storms affecting upper atmosphere (used later if civilization exists).

Cause codes: impact_winter, tsunami_event, geomagnetic_storm.

Validation: short-term cooling and coastal pulses after major impact.

⸻

11) Hazard Composer (Compound Risk)

Combine outputs to create human-relevant hazard flags (even pre-human, for calibration):
	•	Coastal flooding = base sea level + tide envelope + storm surge + river discharge.
	•	Heat-fire combos = temperature anomaly + drought + wind.
	•	Volcano-rain = ash + intense rain → lahar flags.
	•	Quake-tsunami coupling.

Cause codes: compose from inputs, retain provenance chain.

⸻

12) Time & Integration
	•	Cadences (suggested):
Astronomy 1–10y; Atmos/Ocean monthly/seasonal; Hydrology monthly; Cryosphere seasonal/annual; Geodynamics/Erosion 50–100y; BioGeo annual/seasonal.
	•	Operator splitting: advance fast subsystems multiple sub-steps per 5-year narrative tick; keep caches of aggregates (means, maxima, duration above thresholds).
	•	Crisis sub-ticks: when compound flags trigger, temporarily sub-step finer (months) for that frame, then aggregate.

⸻

13) Data Contracts & Diffs (examples to standardize)

All persistent commits are integers or bounded enums.
	•	temp: {"r:12": +7} (tenths °C over baseline for the 5-y frame)
	•	precip: mm/5-y frame
	•	sst: tenths °C
	•	sea_level: mm
	•	albedo: milli-units
	•	biome: enum index
	•	flood_flags: [{"region":12,"severity":0.62,"cause":["tide_spring","storm_surge","rain_runoff"]}]
	•	aod: milli-AOD (global/regional)
	•	npp: gC/m²/5-y

Each event/delta carries a cause[] populated from canonical codes.

⸻

14) Cause Code Lexicon (planetary extensions)

Astronomy: insolation_gradient, obliquity_shift, precession_phase, tide_spring, tide_neap, solar_cycle_peak, cme_event
Atmosphere: hadley_cell, orographic_lift, monsoon_onset, storm_track_shift, enso_phase, aerosol_radiative_forcing
Ocean: wind_gyre_coupling, thermohaline_shift, sea_level_thermal_expansion, storm_surge_setup
Cryosphere: albedo_feedback, glacier_mass_balance, freshwater_pulse, permafrost_thaw
Hydrology: rain_runoff, snowmelt_surge, tide_compound_flood, soil_infiltration_limit, seiche_event
Terrain: fluvial_erosion, coastal_retreat, delta_progradation, mass_wasting_event
Biogeochem: photosynthesis_uptake, respiration_release, erosion_loss, volcanic_outgassing, eutrophication
Extreme: impact_winter, tsunami_event, geomagnetic_storm

(Keep codes short, stable, and composable.)

⸻

15) Calibration, Validation & Testing
	•	Sanity invariants: non-negative stores, bounded indices, conservation in box models, clamped commits.
	•	Anchors:
	•	Hadley edge ~30°; ITCZ at thermal equator.
	•	Monsoon seasonality; polar amplification with higher albedo.
	•	Tide spring/neap periodicity; compound flood frequency rises with tide + surge overlap.
	•	Sea level ↑ with ocean heat + ice loss.
	•	Volcanic AOD → transient cooling/precip suppression.
	•	Method: property tests (bounds), golden runs (small worlds), scenario DSL (“mega-eruption”, “impact winter”, “ENSO-flip”), ensemble sweeps.

⸻

16) Performance & Determinism
	•	Layout: SoA arrays per field; preallocated buffers; region chunking (e.g., 64×64 tiles).
	•	Parallelism: shard per region or basin; stable reduce by sorted indices.
	•	RNG: deterministic substreams (seed, stage, tick); no global RNG.
	•	Commit: fixed-point integers; floats only in intermediate math; clamp/round before commit.
	•	I/O: NDJSON during dev; later MsgPack+zstd for snapshots.

⸻

17) Tooling (keep UI optional)
	•	Godot viewer stays minimal: atlas + metric toggle + scrolling log.
	•	TUI tools:
	•	simtail (filter logs; follow WS or files)
	•	simdump (NDJSON → CSV/Parquet summaries)
	•	simplot (tiny CLI plots for dev)
	•	Scenario DSL to pin stress tests and reproduce edge cases.

⸻

18) Variant Worlds (future seeds)
	•	Tidal-locked (M-dwarf): permanent day/night; strong substellar convection; weak Coriolis.
	•	High-obliquity: extreme seasons; migrating Hadley cells.
	•	Ocean world: minimal land; weak silicate weathering; storm-dominated.
	•	Dry world: closed basins; dust storms; large diurnal swings.
	•	Multiple moons / large moon: strong tides, resonance floods.
	•	Weak magnetosphere: higher atmospheric escape & radiation.

(Architecture stays the same; only seed parameters change.)

⸻

19) Roadmap sketch (beyond v0.1)
	•	v0.2 Astronomy MVP (insolation + tide envelope) + Cryosphere MVP
	•	v0.3 Ocean mixed-layer + gyres; sea level thermal expansion; storm surge proxy
	•	v0.4 Biogeochem 4-box + Köppen biomes; NPP → carrying capacity
	•	v0.5 Terrain/Erosion MVP + coastal retreat; floodplain storage in Hydrology
	•	v0.6 ENSO-like mode; drought teleconnections; wildfire regime
	•	v0.7 Speciation/ecology coupling refinements; disease ecology stubs
	•	v0.8 Volcanic AOD pulses; impact winter stubs; hazard composer v1
	•	v0.9 Performance/determinism hardening; ensemble tooling; scenario DSL 1.0
	•	1.0 Planet “feels right”; then layer cognition/civilization