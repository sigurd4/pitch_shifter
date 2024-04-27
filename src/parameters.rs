use vst::prelude::PluginParameters;
use vst::util::AtomicFloat;

pub const PITCH_PER_FINE_PITCH: f32 = 1.0/12.0;
pub const OCTAVES_PER_UNIT_PITCH: f32 = 1.0;
pub const CENTS_PER_UNIT_PITCH: f32 = 12.0*100.0*OCTAVES_PER_UNIT_PITCH;
pub const PITCH_MAX: f32 = 1.0/OCTAVES_PER_UNIT_PITCH;
pub const PITCH_MIN: f32 = -1.0/OCTAVES_PER_UNIT_PITCH;

#[derive(Clone, Copy)]
pub enum Control
{
    Pitch,
    PitchFine,
    Mix
}

impl Control
{
    pub const VARIANT_COUNT: usize = core::mem::variant_count::<Self>();
    pub const VARIANTS: [Self; Self::VARIANT_COUNT] = [
        Self::Pitch,
        Self::PitchFine,
        Self::Mix
    ];

    pub fn from(i: i32) -> Self
    {
        Self::VARIANTS[i as usize]
    }
}

pub struct BasicFilterParameters
{
    pub pitch: AtomicFloat,
    pub pitch_fine: AtomicFloat,
    pub mix: AtomicFloat
}

impl PluginParameters for BasicFilterParameters
{
    fn get_parameter_label(&self, index: i32) -> String
    {
        match Control::from(index)
        {
            Control::Pitch => "cents".to_string(),
            Control::PitchFine => "cents".to_string(),
            Control::Mix => "%".to_string()
        }
    }

    fn get_parameter_text(&self, index: i32) -> String
    {
        match Control::from(index)
        {
            Control::Pitch => format!("{:.3}", (self.pitch.get() + self.pitch_fine.get()*PITCH_PER_FINE_PITCH)*CENTS_PER_UNIT_PITCH),
            Control::PitchFine => format!("{:.3}", (self.pitch.get() + self.pitch_fine.get()*PITCH_PER_FINE_PITCH)*CENTS_PER_UNIT_PITCH),
            Control::Mix => format!("{:.3}", self.mix.get()*100.0)
        }
    }

    fn get_parameter_name(&self, index: i32) -> String
    {
        match Control::from(index)
        {
            Control::Pitch => "Pitch".to_string(),
            Control::PitchFine => "Pitch (Fine)".to_string(),
            Control::Mix => "Mix".to_string()
        }
    }

    /// Get the value of parameter at `index`. Should be value between 0.0 and 1.0.
    fn get_parameter(&self, index: i32) -> f32
    {
        match Control::from(index)
        {
            Control::Pitch => (self.pitch.get() - PITCH_MIN)/(PITCH_MAX - PITCH_MIN),
            Control::PitchFine => (self.pitch_fine.get() - PITCH_MIN)/(PITCH_MAX - PITCH_MIN),
            Control::Mix => self.mix.get()
        }
    }
    
    fn set_parameter(&self, index: i32, value: f32)
    {
        match Control::from(index)
        {
            Control::Pitch => self.pitch.set(value*(PITCH_MAX - PITCH_MIN) + PITCH_MIN),
            Control::PitchFine => self.pitch_fine.set(value*(PITCH_MAX - PITCH_MIN) + PITCH_MIN),
            Control::Mix => self.mix.set(value)
        }
    }

    fn change_preset(&self, _preset: i32) {}

    fn get_preset_num(&self) -> i32 {
        0
    }

    fn set_preset_name(&self, _name: String) {}

    fn get_preset_name(&self, _preset: i32) -> String {
        "".to_string()
    }

    fn can_be_automated(&self, index: i32) -> bool {
        index < Control::VARIANTS.len() as i32
    }

    fn get_preset_data(&self) -> Vec<u8>
    {
        Control::VARIANTS.map(|v| self.get_parameter(v as i32).to_le_bytes())
            .concat()
    }

    fn get_bank_data(&self) -> Vec<u8>
    {
        self.get_preset_data()
    }

    fn load_preset_data(&self, data: &[u8])
    {
        for (v, &b) in Control::VARIANTS.into_iter()
            .zip(data.array_chunks())
        {
            self.set_parameter(v as i32, f32::from_le_bytes(b));
        }
    }

    fn load_bank_data(&self, data: &[u8])
    {
        self.load_preset_data(data);
    }
}