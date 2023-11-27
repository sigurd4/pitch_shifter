#![feature(split_array)]

use std::{sync::Arc, f32::consts::{TAU, PI}};

use num::Complex;
use parameters::{OCTAVES_PER_UNIT_PITCH, PITCH_PER_FINE_PITCH};
use real_time_fir_iir_filters::{iir::third::ThirdOrderButterworthFilter, Filter};
use sss_fft::{FFTAlgorithmDefault, SlidingDFT};
use vst::{prelude::*, plugin_main};

use self::parameters::{BasicFilterParameters, Control};

pub mod parameters;

const WINDOW_LENGTH: usize = 1024;

struct PitchShifterPlugin
{
    pub param: Arc<BasicFilterParameters>,
    filter: [[ThirdOrderButterworthFilter<f32>; 4]; CHANNEL_COUNT],
    dft: [SlidingDFT<f32, WINDOW_LENGTH>; CHANNEL_COUNT],
    omega: [f32; CHANNEL_COUNT],
    rate: f32
}

const CHANNEL_COUNT: usize = 2;

impl PitchShifterPlugin
{
    fn ifft_once<const N: usize>(omega: f32, x_f: [Complex<f32>; N]) -> f32
    {
        let z = Complex::cis(omega);
        let mut z_n = z;
        x_f[1..N/2].into_iter()
            .map(|x_f| {
                let y = x_f*z_n;
                z_n *= z;
                y.re*2.0
            }).sum::<f32>()/N as f32
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
            filter: [(); CHANNEL_COUNT].map(|()| [(); 4].map(|()| ThirdOrderButterworthFilter::new(rate*PI))),
            dft: [(); CHANNEL_COUNT].map(|()| SlidingDFT::new::<FFTAlgorithmDefault>([0.0; WINDOW_LENGTH])),
            omega: [0.0; CHANNEL_COUNT],
            rate
        }
    }

    fn get_info(&self) -> Info
    {
        Info {
            name: "Pitch Shifter".to_string(),
            vendor: "Soma FX".to_string(),
            presets: 0,
            parameters: Control::CONTROLS.len() as i32,
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
        self.rate = rate;
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>)
    {
        let octaves = (self.param.pitch.get() + self.param.pitch_fine.get()*PITCH_PER_FINE_PITCH)*OCTAVES_PER_UNIT_PITCH;
        let pitch_mul = 2.0f32.powf(octaves);
        let domega_dt = TAU*(pitch_mul - 1.0)/WINDOW_LENGTH as f32;

        let mix = self.param.mix.get();

        const MARGIN: f32 = 0.2;

        {
            let omega_ceil0 = if pitch_mul*2.0f32.powf(MARGIN) > 1.0 {self.rate/pitch_mul*2.0f32.powf(-MARGIN)} else {self.rate}*PI;
            let omega_ceil1 = if pitch_mul*2.0f32.powf(-MARGIN) < 1.0 {self.rate*pitch_mul*2.0f32.powf(-MARGIN)} else {self.rate}*PI;
            let omega_floor0 = self.rate/pitch_mul/(WINDOW_LENGTH/4) as f32*TAU*2.0f32.powf(MARGIN);
            let omega_floor1 = self.rate/(WINDOW_LENGTH/4) as f32*TAU*2.0f32.powf(MARGIN);
            for [filter_low0, filter_low1, filter_high0, filter_high1] in self.filter.iter_mut()
            {
                filter_low0.omega = omega_ceil0;
                filter_low1.omega = omega_ceil1;
                filter_high0.omega = omega_floor0;
                filter_high1.omega = omega_floor1;
            }
        }

        for ((((input_channel, output_channel), [filter_low0, filter_low1, filter_high0, filter_high1]), dft), omega) in buffer.zip()
            .zip(self.filter.iter_mut())
            .zip(self.dft.iter_mut())
            .zip(self.omega.iter_mut())
        {
            for (&input_sample, output_sample) in input_channel.into_iter()
                .zip(output_channel.into_iter())
            {
                let x_f = dft.next(filter_high0.filter(self.rate, filter_low0.filter(self.rate, input_sample)[0])[3]);

                let y = Self::ifft_once(*omega, x_f);

                *output_sample = (1.0 - mix)*input_sample + mix*filter_high1.filter(self.rate, filter_low1.filter(self.rate, y)[0])[3];
                    
                *omega = (*omega + domega_dt) % TAU;
            }
        }
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters>
    {
        self.param.clone()
    }
}

plugin_main!(PitchShifterPlugin);