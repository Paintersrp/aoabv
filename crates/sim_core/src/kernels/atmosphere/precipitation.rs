use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::rng::Stream;
use crate::world::World;

use super::{
    orography::OrographyEffects,
    seasonality::{self, SeasonalityContext},
    HUMIDITY_TEMP_BONUS, HUMIDITY_TENTHS_MAX, LAPSE_RATE_C_PER_KM, MONSOON_HUMIDITY_THRESHOLD,
    MONSOON_STRENGTH_THRESHOLD, PRECIP_MAX_MM, PRECIP_MIN_MM, TEMP_MAX_TENTHS_C, TEMP_MIN_TENTHS_C,
};

pub(super) struct PrecipitationOutcome {
    pub diff: Diff,
    pub chronicle: Vec<String>,
}

pub(super) fn commit(
    world: &World,
    humidity_tenths: &[i32],
    seasonal: &SeasonalityContext,
    orography: &OrographyEffects,
    stream: &Stream,
) -> PrecipitationOutcome {
    let mut diff = Diff::default();
    let mut chronicle = Vec::new();
    let mut monsoon_regions = 0usize;

    for (index, region) in world.regions.iter().enumerate() {
        let mut commit_rng = stream.derive(index as u64);
        let humidity_tenths_value = humidity_tenths[index].clamp(0, HUMIDITY_TENTHS_MAX);
        let humidity_ratio = f64::from(humidity_tenths_value) / f64::from(HUMIDITY_TENTHS_MAX);
        diff.record_humidity(index, humidity_tenths_value);
        let capped_precip = i32::from(region.precipitation_mm).clamp(0, PRECIP_MAX_MM);
        let precip_ratio = f64::from(capped_precip) / f64::from(PRECIP_MAX_MM);
        let insolation_tenths = world
            .climate
            .last_insolation_tenths
            .get(index)
            .copied()
            .unwrap_or(0);

        let effective_latitude =
            (region.latitude_deg - seasonal.hadley_lat_shift).clamp(-90.0, 90.0);
        let hadley = seasonality::hadley_strength(effective_latitude);
        let baseline_offset = world
            .climate
            .temperature_baseline_tenths
            .get(index)
            .copied()
            .unwrap_or(0);
        let mut temperature_tenths = compute_temperature_tenths(
            effective_latitude,
            region.elevation_m,
            humidity_ratio,
            seasonal.insolation_bias,
        )
        .clamp(TEMP_MIN_TENTHS_C, TEMP_MAX_TENTHS_C);
        temperature_tenths = (temperature_tenths + i32::from(baseline_offset))
            .clamp(TEMP_MIN_TENTHS_C, TEMP_MAX_TENTHS_C);
        if i32::from(region.temperature_tenths_c) != temperature_tenths {
            diff.record_temperature(index, temperature_tenths);
        }

        let base_precip = compute_precip_mm(
            effective_latitude,
            region.elevation_m,
            humidity_ratio,
            hadley,
            seasonal.insolation_bias,
        );
        let jitter = (commit_rng.next_f64() - 0.5) * 0.04;
        let scaled_precip =
            (f64::from(base_precip) * orography.precip_multipliers[index] * (1.0 + jitter)).round()
                as i32;
        let precip_mm = scaled_precip.clamp(PRECIP_MIN_MM, PRECIP_MAX_MM);
        if u16::from(region.precipitation_mm) != precip_mm as u16 {
            diff.record_precipitation(index, precip_mm);
        }

        if seasonality::has_seasonal_variation(seasonal.scalar) {
            diff.record_cause(Entry::new(
                format!("region:{}/temperature", region.id),
                Code::SeasonalShift,
                Some(format!("scalar={:.3}", seasonal.scalar)),
            ));
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::SeasonalShift,
                Some(format!("scalar={:.3}", seasonal.scalar)),
            ));
        }

        if seasonality::has_seasonal_variation(seasonal.hadley_lat_shift) {
            diff.record_cause(Entry::new(
                format!("region:{}/temperature", region.id),
                Code::HadleyDrift,
                Some(format!("shift_deg={:.2}", seasonal.hadley_lat_shift)),
            ));
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::HadleyDrift,
                Some(format!("shift_deg={:.2}", seasonal.hadley_lat_shift)),
            ));
        }

        if hadley > 0.0 {
            diff.record_cause(Entry::new(
                format!("region:{}/temperature", region.id),
                Code::HadleyCell,
                Some(format!("strength={:.2}", hadley)),
            ));
        }

        if orography.lift_gradients[index] > 0.0 {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::OrographicLift,
                Some(format!(
                    "gradient_km={:.2};multiplier={:.2}",
                    orography.lift_gradients[index], orography.lift_multipliers[index]
                )),
            ));
        }

        if orography.rain_shadow_factors[index] > 0.0 {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::RainShadow,
                Some(format!(
                    "shadow_factor={:.2}",
                    orography.rain_shadow_factors[index]
                )),
            ));
        }

        diff.record_cause(Entry::new(
            format!("region:{}/humidity", region.id),
            Code::HumidityTransport,
            Some(format!(
                "precip_ratio={:.2};insolation_tenths={}",
                precip_ratio, insolation_tenths
            )),
        ));

        let monsoon_strength = hadley * humidity_ratio;
        if hadley > MONSOON_STRENGTH_THRESHOLD && humidity_ratio >= MONSOON_HUMIDITY_THRESHOLD {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::MonsoonOnset,
                Some(format!("intensity={:.2}", monsoon_strength)),
            ));
            monsoon_regions += 1;
        }
    }

    let summary = if monsoon_regions > 0 {
        format!(
            "Hadley cells shifted {:+.1}°; monsoons intensified across {} regions.",
            seasonal.hadley_lat_shift, monsoon_regions
        )
    } else {
        format!(
            "Hadley cells shifted {:+.1}°; seasonal scalar {:+.2}.",
            seasonal.hadley_lat_shift, seasonal.scalar
        )
    };
    chronicle.push(summary);

    PrecipitationOutcome { diff, chronicle }
}

fn compute_temperature_tenths(
    latitude_deg: f64,
    elevation_m: i32,
    humidity_ratio: f64,
    insolation_bias: f64,
) -> i32 {
    let insolation =
        (seasonality::insolation_factor(latitude_deg) * insolation_bias).clamp(0.0, 1.2);
    let base_temp_c = -25.0 + 60.0 * insolation;
    let lapse = (f64::from(elevation_m.max(0)) / 1_000.0) * LAPSE_RATE_C_PER_KM;
    let humidity_bonus = (humidity_ratio - 0.5) * HUMIDITY_TEMP_BONUS;
    ((base_temp_c - lapse + humidity_bonus) * 10.0).round() as i32
}

fn compute_precip_mm(
    latitude_deg: f64,
    elevation_m: i32,
    humidity_ratio: f64,
    hadley_strength: f64,
    insolation_bias: f64,
) -> i32 {
    let insolation =
        (seasonality::insolation_factor(latitude_deg) * insolation_bias).clamp(0.0, 1.2);
    let elevation_km = f64::from(elevation_m.max(0)) / 1_000.0;
    let lift_bonus = (elevation_km * 260.0).min(700.0);
    let convective = 1_000.0 + 2_200.0 * humidity_ratio * insolation;
    let hadley_bonus = 1_200.0 * hadley_strength * humidity_ratio;
    let humidity_penalty = (1.0 - humidity_ratio).max(0.0) * 700.0;
    let thin_air_penalty = elevation_km.powf(1.15) * 120.0;
    let precip = convective + hadley_bonus + lift_bonus - humidity_penalty - thin_air_penalty;
    precip.round() as i32
}
