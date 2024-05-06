use std::clone;
use std::fmt;
use crate::ring_buffer;
use nih_plug::params::enums::Enum;
use ring_buffer::RingBuffer;
use crate::adsr;
use adsr::{Adsr, AdsrState};
use crate::crossfade;
use crossfade::Crossfade;

#[derive(Clone)]
pub struct SamplerVoice{
    phase_offset: f32,
    phase_step: f32,
    pub midi_note: u8,
    pub base_midi: u8,
    num_channels: usize,
    sample_rate: f32,
    pub adsr: Adsr,
    pub sus_is_velo: bool,
    start_point: f32,
    end_point: f32,
    reversed: bool,
    sus_mode: SustainModes,
    sus_start: f32,
    sus_end: f32,
    crossfader: Crossfade,
    fade_time: f32,
    sus_passed: bool,
    voice_type: VoiceType,
    pub internal_buffer: RingBuffer<f32>
}
#[derive(Clone, Copy, PartialEq, Enum, Debug)]
pub enum SustainModes {
    NoLoop,
    LoopWrap,
    LoopBounce,
}
impl SustainModes {
    pub fn iter() -> impl Iterator<Item = Self> {
        [SustainModes::NoLoop, SustainModes::LoopWrap, SustainModes::LoopBounce].iter().copied()
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum VoiceType{
    Warp,
    Assign,
}


impl SamplerVoice{
    /// Deals with an audio file and either plays it back at a set rate
    /// based on a midi note or plays back an assigned file
    pub fn new(num_channesls_: usize, sample_rate_: f32, base_midi_: u8, voice_type_: VoiceType)->Self{
        let adsr_ = Adsr::new(sample_rate_, 0.2, 0.1,0.5,0.2);
        let fader = Crossfade::new(sample_rate_, 0.0, 0.0);
        SamplerVoice{
            phase_offset: 0.0,
            phase_step: 1.0,
            midi_note: 0,
            base_midi: base_midi_,
            num_channels: num_channesls_,
            sample_rate: sample_rate_,
            adsr: adsr_,
            sus_is_velo: false,
            start_point: 0.0,
            end_point: -1.0,
            reversed: false,
            sus_mode: SustainModes::NoLoop,
            sus_start: -1.0,
            sus_end: -1.0,
            crossfader: fader,
            fade_time: 0.0002,
            sus_passed: false,
            voice_type: voice_type_,
            internal_buffer: RingBuffer::<f32>::new(1)
        }
    }
    ///Reads from the loaded sample file
    /// Uses the get_frac function in the ring_buffer, which returns the sample
    /// at a fractional index
    pub fn process(&mut self, buffer: &mut RingBuffer<f32>, sr_scalar: f32)->f32{
        self.check_inits(buffer.capacity());
        let fade_samps = self.fade_time*self.sample_rate;
        let cross_start;
        if self.adsr.is_active(){
            let mut sample = buffer.get_frac(self.phase_offset);
            if !self.reversed{
                cross_start = self.sus_end - fade_samps;
                self.phase_offset += self.phase_step * sr_scalar;
                if self.sus_mode != SustainModes::NoLoop{
                    self.sus_logic(&mut sample, cross_start);
                }
                if self.phase_offset >= self.end_point{
                    self.phase_step = 0.0;
                    self.phase_offset = self.start_point;
                    return 0.0
                }
            }else{     
                cross_start = self.sus_start + fade_samps;
                self.phase_offset -= self.phase_step * sr_scalar;
                if self.sus_mode != SustainModes::NoLoop{
                    self.sus_logic(&mut sample, cross_start);
                }
                if self.phase_offset <= self.end_point{
                    self.phase_step = 0.0;
                    self.phase_offset = self.start_point;
                    return 0.0
                }
            }
            sample * self.adsr.get_next_sample()
        }else{
            self.phase_offset = self.start_point;
            self.sus_passed = false;
            0.0
        }
    }
    pub fn process_sfz(&mut self, sr_scalar:f32)->f32{
        self.check_inits(self.internal_buffer.capacity());
        let fade_samps = self.fade_time*self.sample_rate;
        let cross_start;
        if self.adsr.is_active(){
            let mut sample = self.internal_buffer.get_frac(self.phase_offset);
            if !self.reversed{
                cross_start = self.sus_end - fade_samps;
                self.phase_offset += self.phase_step * sr_scalar;
                if self.sus_mode != SustainModes::NoLoop{
                    self.sus_logic(&mut sample, cross_start);
                }
                if self.phase_offset >= self.end_point{
                    self.phase_step = 0.0;
                    self.phase_offset = self.start_point;
                    return 0.0
                }
            }else{     
                cross_start = self.sus_start + fade_samps;
                self.phase_offset -= self.phase_step * sr_scalar;
                if self.sus_mode != SustainModes::NoLoop{
                    self.sus_logic(&mut sample, cross_start);
                }
                if self.phase_offset <= self.end_point{
                    self.phase_step = 0.0;
                    self.phase_offset = self.start_point;
                    return 0.0
                }
            }
            sample * self.adsr.get_next_sample()
        }else{
            self.phase_offset = self.start_point;
            self.sus_passed = false;
            0.0
        }
    }
    ///Sets the midi note for the output
    /// 
    /// Is in reference to the base midi note
    pub fn set_note(&mut self, note: u8){
        self.midi_note = note;
        if self.voice_type == VoiceType::Warp{
            let offset = iclamp((note as i8 - self.base_midi as i8)as i32,-127,127);
            self.phase_step = 2.0_f32.powf(offset as f32 / 12.0);
        }else{
            self.phase_step = 1.0;
        }
    }
    /// Triggers attack on ADSR and starts playback of the audio file
    pub fn note_on(&mut self, note: u8, velocity: f32){
        if self.sus_is_velo {
            self.adsr.set_sustain(velocity);
        }
        self.phase_offset = self.start_point;
        self.set_note(note);
        self.adsr.note_on();
    }
    /// Triggers release on ADSR
    pub fn note_off(&mut self){
        self.adsr.note_off()
    }
    /// Sets the attack, decay, sustain, and release for the ADSR (in seconds)
    pub fn set_adsr(&mut self, attack_:f32, decay_:f32, sustain_:f32, release_:f32){
        if !self.sus_is_velo{
            self.adsr.set_sustain(sustain_);
        }
        self.adsr.set_attack(attack_);
        self.adsr.set_decay(decay_);
        self.adsr.set_release(release_);
    }
    /// Sets the point at which the sample begins playing back (0%-99%)
    /// 
    /// If the start point is greater than the endpoint, the playback will be reversed
    pub fn set_start_point(&mut self, start_point: f32, length: usize){
        self.check_inits(length);
        let point = 0.01 * fclamp(start_point, 0.0, 100.0);
        self.start_point = point * length as f32;
        self.reversed =  self.start_point > self.end_point;
    }
    /// Sets the point at which the sample ends playing back (1%-100%)
    /// 
    /// If the end point is less than the start point, the playback will be reversed
    pub fn set_end_point(&mut self, end_point: f32, length: usize){
        self.check_inits(length);
        let point = 0.01 * fclamp(end_point, 0.0, 100.0);
        self.end_point = point * length as f32;
        self.reversed =  self.start_point > self.end_point;
    }
    /// Sets the start point and end point for the sample's playback. 
    /// 
    /// start_point: (0%-100%),     end_point: (0%-100%)
    /// 
    /// If the start point is greater than the endpoint, the playback will be reversed
    pub fn set_start_and_end_point(&mut self, start_point: f32, end_point: f32, length: usize){
        self.set_start_point(start_point,length);
        self.set_end_point(end_point,length);
    }
    /// Sets the start point of the sustain loop. If reversed, start_point will serve
    /// as end_point. Values will be clamped within start and end points of the 
    /// sample as a whole.
    /// 
    /// start_point: (0%-100%)
    pub fn set_sus_start(&mut self, start_point: f32, length: usize){
        self.check_inits(length);
        let mut point = 0.01 * fclamp(start_point, 0.0, 100.0) * length as f32;
        if !self.reversed{
            if point < self.start_point {point = self.start_point;}
            if point >= self.sus_end {point = self.sus_end-10.0}
            if point >= self.end_point {point = self.end_point-10.0}
        }else{
            if point < self.end_point {point = self.end_point;}
            if point >= self.sus_end {point = self.sus_end-10.0}
            if point >= self.start_point {point = self.start_point-10.0}
        }
        self.sus_start = point;
    }
    /// Sets the end point of the sustain loop. If reversed, end_point will serve
    /// as start_point. Values will be clamped within start and end points of the 
    /// sample as a whole.
    /// 
    /// end_point: (0%-100%)
    pub fn set_sus_end(&mut self, end_point: f32, length: usize){
        self.check_inits(length);
        let mut point = 0.01 * fclamp(end_point, 0.0, 100.0) * length as f32;
        if !self.reversed{
            if point <= self.start_point {point = self.start_point+10.0;}
            if point <= self.sus_start {point = self.sus_start+10.0}
            if point > self.end_point {point = self.end_point}
        }else{
            if point <= self.end_point {point = self.end_point+10.0;}
            if point <= self.sus_start {point = self.sus_end+10.0}
            if point > self.start_point {point = self.start_point}
        }
        self.sus_end = point;
    }
    /// Sets the start and end points for the sustain loop. If reversed, end_point will serve
    /// as start_point. Values will be clamped within start and end points of the 
    /// sample as a whole.
    /// 
    /// start_point: (0%-100%),  end_point: (0%-100%)
    pub fn set_sus_points(&mut self, start_point: f32, end_point: f32, length: usize){
        self.set_sus_start(start_point, length);
        self.set_sus_end(end_point, length);
    }
    /// Returns a tuple containing the start and end points (in percent) of the sampler voice
    /// 
    /// Returns in the format: (start_point, end_point)
    pub fn get_points(&mut self, length: usize)-> (f32, f32){
        let start_point = self.start_point * 100.0 / length as f32;
        let end_point  = self.end_point * 100.0 / length as f32;
        (start_point,end_point)
    }
    /// Returns a tuple containing the start and end points (in percent) of the sustain loop
    /// 
    /// Returns in the format: (start_point, end_point)
    pub fn get_sus_points(&mut self, length: usize)->(f32, f32){
        let start_point = self.sus_start * 100.0 / length as f32;
        let end_point  = self.sus_end * 100.0 / length as f32;
        (start_point,end_point)
    }
    /// Returns whether or not the ADSR is active.
    /// 
    /// Useful for voice allocation
    pub fn is_active(&mut self)->bool{
        self.adsr.is_active()
    }
    /// Sets center midi note upon which sample warping is wrapped
    pub fn set_base_midi(&mut self, note: u8){
        self.base_midi = note;
    }
    /// Sets crossfade time in seconds, expects values between (0.00001 and 0.1)
    pub fn set_fade_time(&mut self, fade_time: f32){
        self.fade_time = fclamp(fade_time, 0.0, 0.1);
        self.crossfader.set_values(fade_time*0.5, fade_time*0.5);
    }
    /// Sets the sustain mode
    pub fn set_sus_loop_mode(&mut self, mode: SustainModes){
        self.sus_mode = mode;
    }
    /// Makes sure there are proper initial values if none have been assigned
    fn check_inits(&mut self, capacity: usize){
        if self.end_point == -1.0{
            self.end_point = capacity as f32;
        }
        if self.sus_start == -1.0{
            self.sus_start = 0.4 * capacity as f32;
        }
        if self.sus_end == -1.0{
            self.sus_end = 0.6 * capacity as f32;
        }
    }
    /// Handles the logic for the different sustain looping modes
    fn sus_logic(&mut self, sample: &mut f32, cross_start: f32){
        if self.adsr.state == AdsrState::Sustain{
            if self.sus_mode == SustainModes::LoopWrap{
                 if !self.reversed{
                     if self.phase_offset >= cross_start && self.phase_offset <= cross_start+self.phase_step{
                         self.crossfader.start_fade_out();
                     }
                     if self.phase_offset >= self.sus_end{
                         self.phase_offset = self.sus_start;
                         self.crossfader.start_fade_in();
                     }
                     if self.phase_offset >= self.sus_start{
                         *sample *= self.crossfader.get_next_sample();
                     }
                 } else{
                     if self.phase_offset <= cross_start && self.phase_offset >= cross_start-self.phase_step{
                         self.crossfader.start_fade_out();
                     }
                     if self.phase_offset <= self.sus_start{
                         self.phase_offset = self.sus_end;
                         self.crossfader.start_fade_in();
                     }
                     if self.phase_offset <= self.sus_end{
                         *sample *= self.crossfader.get_next_sample();
                     }
                 }
            }else if self.sus_mode == SustainModes::LoopBounce {
                if !self.reversed{
                    if !self.sus_passed && self.phase_offset >= self.sus_start{
                        self.sus_passed = true;
                    }
                    if self.phase_offset >= self.sus_end{
                        self.phase_step *= -1.0;
                    }
                    if self.sus_passed && self.phase_offset <= self.sus_start{
                        self.phase_step *= -1.0;
                    }
                }else{
                    if !self.sus_passed && self.phase_offset <= self.sus_end{
                        self.sus_passed = true;
                    }
                    if self.phase_offset <= self.sus_start{
                        self.phase_step *= -1.0;
                    }
                    if self.sus_passed && self.phase_offset >= self.sus_end{
                        self.phase_step *= -1.0;
                    }
                }
            }
        }else{
            if self.phase_step < 0.0 { // makes sure increment isnt negative after sustain looping
                self.phase_step *= -1.0;
            }
        }
    }
}


/// Clamps floats between a min and a max
fn fclamp(x: f32, min_val: f32, max_val: f32) -> f32 {
    if x < min_val {
        min_val
    } else if x > max_val {
        max_val
    } else {
        x
    }
}
/// Clamps ints between a min and a max
fn iclamp(x: i32, min_val: i32, max_val: i32) -> i32 {
    if x < min_val {
        min_val
    } else if x > max_val {
        max_val
    } else {
        x
    }
}
