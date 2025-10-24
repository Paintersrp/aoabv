# AoABV Data Catalog (v0.1)

This catalog defines authoritative sources we may integrate. Each dataset entry is a **YAML code block** with required fields. The datafetch tool parses these blocks to validate and produce a deterministic manifest.

## Required fields per entry
- `id`: short unique key, kebab-case.
- `provider`: organization or program.
- `product`: dataset name/version.
- `version`: pinned dataset version or DOI tag.
- `format`: expected primary distribution format(s).
- `access`:
  - `method`: one of `http`, `https`, `ftp`, `s3`, `gs`, `earthdata`, `cmems`, `local`
  - `url`: canonical landing or API root (informational only in v0.1; no fetch in CI)
  - `auth`: `none` | `earthdata` | `copernicus`
- `spatial_resolution`: human string (e.g., "0.25°", "15 arc-sec", "≈1 km").
- `temporal_coverage`: human string (e.g., "1979–present", "static").
- `variables`: list of variable groups or key variables.
- `license`: short note or URL.
- `sample`: boolean indicating whether this dataset participates in **sample mode** (true keeps CI small).
- `notes`: brief usage/normalization notes.

> Future: When we enable real downloads, we will extend entries with `subset` (bbox, grid), and `artifacts` describing concrete files with checksums.

---

### Dataset entries

```yaml
id: era5-reanalysis
provider: ECMWF Copernicus (C3S)
product: ERA5 hourly single levels
version: 1
format: [NetCDF, GRIB]
access:
  method: https
  url: https://cds.climate.copernicus.eu/
  auth: copernicus
spatial_resolution: 0.25°
temporal_coverage: 1940–present
variables: [2m_temperature, total_precipitation, 10m_wind_u, 10m_wind_v, msl]
license: Copernicus open license (attribution)
sample: false
notes: Primary historical forcing candidate; pick one reanalysis to minimize bias/complexity.
```

```yaml
id: merra2-reanalysis
provider: NASA GMAO (GES DISC)
product: MERRA-2 hourly 2D surface fluxes
version: 5.12.4
format: [NetCDF4]
access:
  method: https
  url: https://disc.gsfc.nasa.gov/
  auth: earthdata
spatial_resolution: 0.5° x 0.625°
temporal_coverage: 1980–present
variables: [T2M, PRECTOTCORR, U10M, V10M, PS]
license: NASA EOSDIS open (registration)
sample: false
notes: Alternate forcing; useful for cross-checking ERA5-derived fields.
```

```yaml
id: gpcp-daily
provider: GPCP
product: GPCP V1.3 Daily 1° Precipitation
version: 1.3
format: [NetCDF]
access:
  method: https
  url: https://psl.noaa.gov/data/gridded/data.gpcp.html
  auth: none
spatial_resolution: 1°
temporal_coverage: 1996–present
variables: [precipitation]
license: Open (cite GPCP)
sample: true
notes: Precip validation/benchmark against reanalysis precipitation.
```

```yaml
id: srtm-dem
provider: USGS
product: SRTM Void-Filled DEM
version: 4.1
format: [GeoTIFF]
access:
  method: https
  url: https://earthexplorer.usgs.gov/
  auth: none
spatial_resolution: 30 m / 90 m
temporal_coverage: static
variables: [elevation]
license: Public domain (USGS)
sample: true
notes: Use as canonical terrain; resample to sim grid. Sample mode will not download tiles.
```

```yaml
id: gebco-2024
provider: GEBCO
product: GEBCO_2024 Grid (bathymetry)
version: 2024
format: [NetCDF, GeoTIFF]
access:
  method: https
  url: https://www.gebco.net/data_and_products/gridded_bathymetry_data/
  auth: none
spatial_resolution: 15 arc-sec
temporal_coverage: static
variables: [bathymetry]
license: Open (see GEBCO terms)
sample: true
notes: Ocean floor topography; stitch with land DEM if needed.
```

```yaml
id: worldclim-v2-1
provider: WorldClim
product: Bioclim variables
version: 2.1
format: [GeoTIFF]
access:
  method: https
  url: https://www.worldclim.org/data/worldclim21.html
  auth: none
spatial_resolution: ≈1 km
temporal_coverage: 1970–2000 climatology (static)
variables: [bio1, bio5, bio12]
license: Open with attribution
sample: true
notes: Baseline climatologies; handy for sanity checks and visualization basemaps.
```

```yaml
id: hydrobasins
provider: WWF HydroSHEDS
product: HydroBASINS levelled basins
version: 1.0
format: [Shapefile, GeoPackage]
access:
  method: https
  url: https://www.hydrosheds.org/products/hydrobasins
  auth: none
spatial_resolution: vector topology
temporal_coverage: static
variables: [basin_id, level, nextdown]
license: CC BY 4.0
sample: true
notes: Routing/catchment topology; snap to resampled DEM.
```

```yaml
id: ghsl
provider: European Commission JRC
product: Global Human Settlement Layer
version: 2023
format: [GeoTIFF]
access:
  method: https
  url: https://ghsl.jrc.ec.europa.eu/
  auth: none
spatial_resolution: 30 m–1 km (varies)
temporal_coverage: multi-epoch
variables: [built, population]
license: Open (JRC data policy)
sample: false
notes: Optional human-coupling overlays; out of scope for v0.1 fetch.
```

```
