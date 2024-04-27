#![feature(split_array)]
#![feature(variant_count)]
#![feature(array_chunks)]

use std::{sync::Arc, f64::consts::{TAU, PI}};

use array_math::ArrayMath;
use num::{Complex, Float, Zero};
use parameters::{OCTAVES_PER_UNIT_PITCH, PITCH_PER_FINE_PITCH};
use real_time_fir_iir_filters::{iir::third::ThirdOrderButterworthFilter, Filter};
use signal_processing::Sdft;
use vst::{prelude::*, plugin_main};

use self::parameters::{BasicFilterParameters, Control};

pub mod parameters;

const WINDOW_LENGTH: usize = 1024;

const F_ANTI_POP: f64 = 10000.0;

const CHANNEL_COUNT: usize = 2;

struct PitchShifterPlugin
{
    pub param: Arc<BasicFilterParameters>,
    anti_alias_filter: [[ThirdOrderButterworthFilter<f64>; 4]; CHANNEL_COUNT],
    anti_pop_filter: [ThirdOrderButterworthFilter<f64>; CHANNEL_COUNT],
    dft: [([Complex<f64>; WINDOW_LENGTH], Vec<f64>); CHANNEL_COUNT],
    omega: [f64; CHANNEL_COUNT],
    pitch_mul: f64,
    rate: f64
}

impl PitchShifterPlugin
{
    fn ifft_once<const N: usize>(omega: f64, x_f: [Complex<f64>; N]) -> f64
    {
        let z = Complex::cis(omega);
        let mut z_n = z;
        (x_f[0].re + x_f[1..N/2 + 1].into_iter()
            .map(|x_f| {
                let y = x_f*z_n;
                z_n *= z;
                y.re*2.0
            }).sum::<f64>())/N as f64
    }
    
    fn process<F>(&mut self, buffer: &mut AudioBuffer<F>)
    where
        F: Float
    {
        let octaves = ((self.param.pitch.get() + self.param.pitch_fine.get()*PITCH_PER_FINE_PITCH)*OCTAVES_PER_UNIT_PITCH) as f64;
        let pitch_mul = 2.0f64.powf(octaves);
        let domega_dt = TAU*(pitch_mul - 1.0)/WINDOW_LENGTH as f64;

        let mix = self.param.mix.get() as f64;

        const MARGIN: f64 = 0.2;

        if pitch_mul != self.pitch_mul
        {
            let omega_ceil0 = if pitch_mul*2.0f64.powf(MARGIN) > 1.0 {self.rate/pitch_mul*2.0f64.powf(-MARGIN)} else {self.rate}*PI;
            let omega_ceil1 = if pitch_mul*2.0f64.powf(-MARGIN) < 1.0 {self.rate*pitch_mul*2.0f64.powf(-MARGIN)} else {self.rate}*PI;
            let omega_floor0 = self.rate/pitch_mul/(WINDOW_LENGTH/8) as f64*TAU*2.0f64.powf(MARGIN);
            let omega_floor1 = self.rate/(WINDOW_LENGTH/8) as f64*TAU*2.0f64.powf(MARGIN);
            for [filter_low0, filter_low1, filter_high0, filter_high1] in self.anti_alias_filter.iter_mut()
            {
                filter_low0.omega = omega_ceil0;
                filter_low1.omega = omega_ceil1;
                filter_high0.omega = omega_floor0;
                filter_high1.omega = omega_floor1;
            }
            self.pitch_mul = pitch_mul;
        }

        for (((((input_channel, output_channel), [filter_low0, filter_low1, filter_high0, filter_high1]), anti_pop_filter), dft), omega) in buffer.zip()
            .zip(self.anti_alias_filter.iter_mut())
            .zip(self.anti_pop_filter.iter_mut())
            .zip(self.dft.iter_mut())
            .zip(self.omega.iter_mut())
        {
            for (input_sample, output_sample) in input_channel.into_iter()
                .zip(output_channel.into_iter())
            {
                let x = input_sample.to_f64().unwrap();
                let [z, _, _, _] = filter_low0.filter(self.rate, x);
                let [_, _, _, z] = filter_high0.filter(self.rate, z);
                dft.0.sdft(&mut [z], &mut dft.1);

                let y = Self::ifft_once(*omega, dft.0);
                let [y, _, _, _] = filter_low1.filter(self.rate, y);
                let [_, _, _, y] = filter_high1.filter(self.rate, y);
                let [y, _, _, _] = anti_pop_filter.filter(self.rate, y);

                *output_sample = F::from((1.0 - mix)*x + mix*y).unwrap();
                    
                *omega = (*omega + domega_dt + TAU) % TAU;
            }
        }
    }
}

impl Plugin for PitchShifterPlugin
{
    fn new(_host: HostCallback) -> Self
    where
        Self: Sized
    {
        let rate = 44100.0;
        PitchShifterPlugin {
            param: Arc::new(BasicFilterParameters {
                pitch: AtomicFloat::from(0.0),
                pitch_fine: AtomicFloat::from(0.0),
                mix: AtomicFloat::from(1.0)
            }),
            anti_alias_filter: [(); CHANNEL_COUNT].map(|()| [(); 4].map(|()| ThirdOrderButterworthFilter::new(rate*PI))),
            anti_pop_filter: [(); CHANNEL_COUNT].map(|()| ThirdOrderButterworthFilter::new(F_ANTI_POP*TAU)),
            dft: [(); CHANNEL_COUNT].map(|()| ([Complex::zero(); WINDOW_LENGTH], vec![])),
            omega: [0.0; CHANNEL_COUNT],
            pitch_mul: f64::NAN,
            rate
        }
    }

    fn get_info(&self) -> Info
    {
        Info {
            name: "Pitch Shifter".to_string(),
            vendor: "Soma FX".to_string(),
            presets: 0,
            parameters: Control::VARIANTS.len() as i32,
            inputs: CHANNEL_COUNT as i32,
            outputs: CHANNEL_COUNT as i32,
            midi_inputs: 0,
            midi_outputs: 0,
            unique_id: 976359654,
            version: 1,
            category: Category::Effect,
            initial_delay: 0,
            preset_chunks: false,
            f64_precision: true,
            silent_when_stopped: true,
            ..Default::default()
        }
    }

    fn set_sample_rate(&mut self, rate: f32)
    {
        self.rate = rate as f64;
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters>
    {
        self.param.clone()
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>)
    {
        self.process(buffer)
    }

    fn process_f64(&mut self, buffer: &mut AudioBuffer<f64>)
    {
        self.process(buffer)
    }
}

plugin_main!(PitchShifterPlugin);