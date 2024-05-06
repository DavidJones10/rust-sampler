#[derive(Clone)]
pub struct Adsr{
    atk_step: f32,
    dec_step: f32,
    rel_step: f32,
    atk_value: f32,
    dec_value: f32,
    sus_value: f32,
    rel_value: f32,
    sample_rate: f32,
    pub envelope_value: f32,
    pub state: AdsrState,
}
#[derive(PartialEq, Debug, Clone)]
pub enum AdsrState{
    Attack,
    Decay,
    Sustain,
    Release,
    Inactive
}

impl Adsr{
    /// Creates a new ADSR object
    pub fn new(sample_rate_: f32, attack_:f32, decay_:f32, sustain_:f32, release_:f32)->Self{
        
        let mut adsr = Adsr{
            atk_step: 0.0,
            dec_step: 0.0,
            rel_step: 0.0,
            atk_value: 0.1,
            dec_value: 0.1,
            sus_value: 1.0,
            rel_value: 0.1,
            sample_rate: sample_rate_,
            envelope_value: 0.0,
            state: AdsrState::Inactive,
        };
        adsr.set_adsr(attack_, decay_, sustain_, release_);
        adsr
    }
    /// Gets the next sample along the ADSR graph.
    /// Should be called for every sample
    pub fn get_next_sample(&mut self)->f32{
        match self.state{
            AdsrState::Inactive => 0.0,
            AdsrState::Attack => {
                self.envelope_value += self.atk_step;
                if self.envelope_value >= 1.0{
                    self.envelope_value = 1.0;
                    self.get_next_state();
                }
                self.envelope_value
            },
            AdsrState::Decay => {
                self.envelope_value -= self.dec_step;
                if self.envelope_value <= self.sus_value{
                    self.envelope_value = self.sus_value;
                    self.get_next_state();
                }
                self.envelope_value
            },
            AdsrState::Sustain => {
                self.sus_value
            },
            AdsrState::Release => {
                self.envelope_value -= self.rel_step;
                if self.envelope_value <= 0.0{
                    self.envelope_value = 0.0;
                    self.get_next_state();
                }
                self.envelope_value
            }
        } 
    }
    /// Sets attack in seconds
    pub fn set_attack(&mut self, attack_:f32){

        if attack_ <= 0.0{
            self.atk_value = 0.00001;
        }else{
            self.atk_value = attack_;
        }
        self.atk_step = self.get_step(1.0, self.atk_value);
    }
    /// Sets decay in seconds
    pub fn set_decay(&mut self, decay_:f32){
        if decay_ < 0.0{
            self.dec_value = 0.0;
        }else{
            self.dec_value = decay_;
        }
        let dist = 1.0-self.sus_value;
        self.dec_step = self.get_step(dist, self.dec_value);
    }
    /// Sets sustain from 0 to 1
    pub fn set_sustain(&mut self, sustain_:f32){
        self.sus_value = fclamp(sustain_,0.0,1.0);
    }
    /// Sets release in seconds
    pub fn set_release(&mut self, release_:f32){
        if release_ <= 0.0{
            self.rel_value = 0.00001;
        }else{
            self.rel_value = release_;
        }
        self.rel_step = self.get_step(self.sus_value, self.rel_value);
    }
    /// Sets attack, decay, sustain, and release (A,D, and R are in seconds; S: 0-1)
    pub fn set_adsr(&mut self, attack_:f32, decay_:f32, sustain_:f32, release_:f32){
        self.set_sustain(sustain_);
        self.set_attack(attack_);
        self.set_decay(decay_);
        self.set_release(release_);
    }
    pub fn get_adsr(&mut self)->(f32, f32, f32, f32){
        (self.atk_value, self.dec_value, self.sus_value, self. rel_value)
    }
    /// Triggers the attack stage of the ADSR
    pub fn note_on(&mut self){
        self.state = AdsrState::Attack;
    } 
    /// Triggers the release stage of the ADSR
    pub fn note_off(&mut self){
        self.state = AdsrState::Release;

    }
    /// Returns whether or not the ADSR is active (in any of the states: A, D, S, or R)
    pub fn is_active(&mut self)-> bool{
        self.state != AdsrState::Inactive
    }
    /// Returns a step size to draw a line of a certain vertical 'distance' (amplitude) 
    /// in a certain amount of time
    fn get_step(&mut self, distance: f32, time_sec: f32)->f32{
        if time_sec > 0.0 {
            distance / (time_sec*self.sample_rate)
        }else{
            -1.0
        }
    }
    /// Decides what state of the ADSR to go to next
    fn get_next_state(&mut self){
        match self.state{
            AdsrState::Inactive => self.state = AdsrState::Inactive,
            AdsrState::Attack => {
                if self.dec_value > 0.0{
                    self.state = AdsrState::Decay;
                }else{
                    self.state = AdsrState::Sustain;
                }
            },
            AdsrState::Decay => {
                self.state = AdsrState::Sustain
            },
            AdsrState::Sustain => {
                self.state = AdsrState::Release
            },
            AdsrState::Release => {
                self.reset();
            }
        } 
    }
    pub fn reset(&mut self){
        self.state = AdsrState::Inactive;
        self.envelope_value = 0.0;
    }

}

/// Clamps floats between a min and a max value
fn fclamp(x: f32, min_val: f32, max_val: f32) -> f32 {
    if x < min_val {
        min_val
    } else if x > max_val {
        max_val
    } else {
        x
    }
}


#[cfg(test)]
mod tests{
    use super::*;  
    macro_rules! assert_close {
        ($left:expr, $right:expr, $epsilon:expr) => {{
            let (left, right, epsilon) = ($left, $right, $epsilon);
            assert!(
                (left - right).abs() <= epsilon,
                "{} is not close to {} within an epsilon of {}",
                left,
                right,
                epsilon
            );
        }};
    } 
    #[test]
    fn test_functionality(){
        let mut adsr = Adsr::new(50.0, 0.2, 0.1,0.5,0.2);
        let mut signal: Vec<f32> = vec![1.0;50];
        for (i,sample) in signal.iter_mut().enumerate(){
            if i == 0{
                adsr.note_on();
            }
            if i == 40{
                adsr.note_off();
            }
            *sample *= adsr.get_next_sample();
            if i < 10 {
                assert_close!(*sample, i as f32*0.1 +0.1, 0.001);
            }else if i > 9 && i < 15{
                assert_close!(*sample, 1.0 - (i-9)as f32*0.1, 0.001);
            }else if i > 14 && i < 40{
                assert_eq!(*sample, 0.5);
            }else{
                assert_close!(*sample, 0.5 - (i-39)as f32*0.05, 0.001);
            }
        }
    }
}

