#[derive(Clone)]
pub struct Crossfade{
    in_time: f32,
    out_time: f32,
    in_step: f32,
    out_step: f32,
    sample_rate: f32,
    fade_value: f32,
    state: FaderState
}
#[derive(PartialEq, Debug, Clone)]
pub enum FaderState{
    NoEffect, // Fader returns 1.0
    FadingIn,
    FadingOut,
}

impl Crossfade{
    pub fn new(sample_rate_: f32, in_time_: f32, out_time_: f32)->Self{
       let mut fader = Crossfade{
        in_time: 0.0,
        out_time: 0.0,
        in_step: 0.0,
        out_step: 0.0,
        sample_rate: sample_rate_,
        fade_value: 0.0,
        state: FaderState::NoEffect,
       };
       fader.set_values(in_time_, out_time_);
       fader
    }
    pub fn get_next_sample(&mut self)->f32{
        match self.state{
            FaderState::NoEffect => {1.0},
            FaderState::FadingIn =>{
                if self.in_step == -1.0{
                    self.state = FaderState::NoEffect;
                    return 1.0;
                }
                self.fade_value += self.in_step;
                if self.fade_value >= 1.0{
                    self.fade_value = 1.0;
                    self.state = FaderState::NoEffect;
                }
                self.fade_value
            },
            FaderState::FadingOut =>{
                if self.fade_value <= 0.0 || self.out_step == -1.0{
                   return  0.0
                }
                self.fade_value -= self.out_step;
                if self.fade_value <= 0.0{
                    self.fade_value = 0.0;
                }
                self.fade_value
            }
        }
    }
    /// Set fade in and fade out time in seconds
    pub fn set_values(&mut self, in_time_: f32, out_time_: f32){
        self.in_time = fclamp(in_time_, 0.0, 10.0);
        self.out_time = fclamp(out_time_, 0.0, 10.0);
        self.in_step = self.get_step(1.0, self.in_time);
        self.out_step = self.get_step(1.0, self.out_time);
    }
    /// Trigger to start the fade in
    pub fn start_fade_in(&mut self){
        self.state = FaderState::FadingIn;
        self.fade_value = 0.0;
    }
    /// Trigger to start the fade out
    pub fn start_fade_out(&mut self){
        self.state = FaderState::FadingOut;
        self.fade_value = 1.0;
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