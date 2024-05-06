use crate::{sampler_voice,ring_buffer,adsr};
use sampler_voice::{SamplerVoice,SustainModes,VoiceType};
use ring_buffer::RingBuffer;
use std::{collections::HashMap, path::Path};
use hound::SampleFormat;
use adsr::AdsrState;
use sofiza::{Instrument, Opcode};

#[derive(Clone)]
pub struct SamplerEngine{
    num_voices: u8,
    sound_bank: HashMap<u8,(String,f32,RingBuffer<f32>, SamplerVoice)>,
    file_names: Vec<String>,
    warp_buffer: RingBuffer<f32>,
    sampler_mode: SamplerMode,
    warp_voices: Vec<SamplerVoice>,
    sample_rate: f32,
    num_channels: usize,
    warp_sr_scalar: f32,
    instrument: Instrument,
}
#[derive(PartialEq,Clone)]
pub enum SamplerMode{
    Warp, // For when you just load one sample and want it to be pitch warped
    Assign, // For when you load multiple samples and assign them to midi notes
    Sfz, // For when you load an sfz file
}

impl SamplerEngine{
    pub fn new(sample_rate_: f32, num_channels_: usize) -> Self{
        
        let files = vec!["".to_string();100];
        let buff = RingBuffer::<f32>::new(1);
        let voices_ = vec![SamplerVoice::new(num_channels_,sample_rate_,64,VoiceType::Warp);6];

        let mut engine = SamplerEngine{
            num_voices: 6,
            sound_bank: HashMap::with_capacity(30),
            file_names: files,
            warp_buffer: buff,
            sampler_mode: SamplerMode::Warp,
            warp_voices: voices_,
            sample_rate: sample_rate_,
            num_channels: num_channels_,
            warp_sr_scalar: sample_rate_,
            instrument: Instrument::new(),
        };
        engine.file_names.clear();
        engine
    }
    pub fn process(&mut self)->f32{
        let mut out_samp = 0.0;
        match self.sampler_mode{
            SamplerMode::Warp =>{
                for voice in self.warp_voices.iter_mut(){
                    out_samp += voice.process(&mut self.warp_buffer, 
                                                self.warp_sr_scalar);
                }
            },
            SamplerMode::Assign =>{
                for (_note, (_name,sr_scalar,buff,voice)) in self.sound_bank.iter_mut(){
                    out_samp += voice.process(buff,*sr_scalar);
                }
            },
            SamplerMode::Sfz =>{
                for voice in self.warp_voices.iter_mut(){
                    out_samp += voice.process_sfz(self.warp_sr_scalar);
                }
            }
        }
        out_samp
    }
    ///Add a file to the paths of files saved in the file names
    /// and load file into the warp buffer.
    pub fn add_to_paths_and_load(&mut self, file_path: &str){
        if file_path.ends_with(".wav"){
            self.warp_sr_scalar =  fill_warp_buffer(&mut self.warp_buffer, file_path)/
                                self.sample_rate;
            self.file_names.push(file_path.to_string());
        }
    }
    ///Add a file to the paths of files saved in the file names.
    pub fn add_file_to_paths(&mut self, file_path: &str){
        if file_path.ends_with(".wav"){
            self.file_names.push(file_path.to_string());
        }
    }
    ///Load a file into the warp buffer from the list of filepaths that have been added
    /// 
    /// idx will wrap around the size of the file_paths buffer
    pub fn load_file_by_index(&mut self, idx: usize){
        if self.file_names.len() > 0{
            let new_idx = idx % self.file_names.len();
            if let Some(file_path) = self.file_names.get(new_idx){
                self.warp_sr_scalar = fill_warp_buffer(&mut self.warp_buffer, &file_path)/self.sample_rate;
            }
        }
    }

    pub fn get_file_name_by_index(&mut self, idx: usize)->Option<String>{
        let new_idx = idx % self.file_names.len();
        self.file_names.get(new_idx).cloned()
    }
    ///Load file from path into the warp buffer without loading 
    /// into the file names.
    pub fn load_file_from_path(&mut self, file_path: &str){
        self.warp_sr_scalar =  fill_warp_buffer(&mut self.warp_buffer, file_path)/
                                self.sample_rate;
    }
    /// Assigns an audio file to a midi note for the sound bank. (Assign mode)
    /// 
    /// Will add file to paths if not already there
    pub fn assign_file_to_midi(&mut self, file_path: &str, note: u8){
        if !self.file_names.contains(&file_path.to_string()){
            self.add_file_to_paths(file_path);
        }
        let (buff,sr) = create_buffer(file_path);
        let sr_scalar = sr / self.sample_rate;
        self.sound_bank.insert(note,(file_path.to_string(),sr_scalar,buff,
                            SamplerVoice::new(self.num_channels,self.sample_rate,note,VoiceType::Assign))); 
    }

    /// Load an SFZ file and create an instrument
    pub fn load_sfz(&mut self, file_path: &str){
        let result = Instrument::from_file(Path::new(file_path));
        match result {
            Ok(instrument) => self.instrument = instrument,
            Err(_e) => {}
        }
    }

    /// Triggers a "note on" message and allocates a voice, 
    ///  stealing if necessary
    pub fn note_on(&mut self, note: u8, velocity: f32){
        match self.sampler_mode {
            SamplerMode::Warp =>{
                let voice_id = self.get_voice_id();
                self.warp_voices[voice_id].note_on(note, velocity);
            },
            SamplerMode::Assign =>{
                for (_note_, (_name,_sr_scalar,_buff,voice)) in self.sound_bank.iter_mut(){
                    if voice.base_midi == note{
                        voice.note_on(note, velocity);
                        break;
                    }
                } 
            },
            SamplerMode::Sfz =>{
                let instrument = self.instrument.clone();
                for region in instrument.regions.iter(){
                    let mut lokey = u8::MIN;
                    let mut hikey = u8::MAX;
                    let mut lovel = f32::MIN;
                    let mut hivel = f32::MAX;
                    match region.opcodes.get("lokey") {
                        Some(value) => {
                            match value {
                                Opcode::lokey(value) => {
                                    lokey = *value;
                                },
                                _ => println!("Something else")
                            }
                        },
                        None => {}
                    }
                    match region.opcodes.get("hikey") {
                        Some(value) => {
                            match value {
                                Opcode::hikey(value) => {
                                    hikey = *value;
                                },
                                _ => println!("Something else")
                            }
                        },
                        None => {}
                    }
                    match region.opcodes.get("lovel") {
                        Some(value) => {
                            match value {
                                Opcode::lovel(value) => {
                                    lovel = *value as f32;
                                },
                                _ => println!("Something else")
                            }
                        },
                        None => {}
                    }
                    match region.opcodes.get("hivel") {
                        Some(value) => {
                            match value {
                                Opcode::hivel(value) => {
                                    hivel = *value as f32;
                                },
                                _ => println!("Something else")
                            }
                        },
                        None => {}
                    }
                    // Conditional filters
                    if note >= lokey && note <= hikey && velocity*127.0 >= lovel && velocity*127.0 <= hivel {
                        let voice_id = self.get_voice_id();
                        match region.opcodes.get("sample") {
                            Some(value) => {
                                match value {
                                    Opcode::sample(value) => {
                                        match value.to_str() {
                                            Some(file_path) => {
                                                let result = create_buffer(file_path);
                                                self.warp_sr_scalar = result.1/self.sample_rate;
                                                self.warp_voices[voice_id].internal_buffer = result.0.clone();
                                            },
                                            None => { panic!("Could not convert value to string") }
                                        }
                                    },
                                    _ => println!("Something else")
                                }
                            },
                            None => {}
                        }
                        match region.opcodes.get("pitch_keycenter") {
                            Some(value) => {
                                match value {
                                    Opcode::pitch_keycenter(value) => {
                                        self.warp_voices[voice_id].base_midi = *value;
                                    },
                                    _ => println!("Something else")
                                }
                            },
                            None => {}
                        }
                        self.warp_voices[voice_id].note_on(note, velocity);
                    }
                }
            }
        }
    }
    /// Triggers a note off message
    pub fn note_off(&mut self, note: u8){
        match self.sampler_mode {
            SamplerMode::Warp =>{
                for voice in self.warp_voices.iter_mut(){
                    if voice.midi_note == note{
                        voice.note_off();
                        break;
                    }
                }
            },
            SamplerMode::Assign =>{
                for (_note, (_name,_sr_scalar,_buff,voice)) in self.sound_bank.iter_mut(){
                    if voice.base_midi == note{
                        voice.note_off();
                        break;
                    }
                }               
            },
            SamplerMode::Sfz =>{
                for voice in self.warp_voices.iter_mut(){
                    if voice.midi_note == note{
                        voice.note_off();
                        break;
                    }
                }
            }
        }
    }
    /// Sets the attack, decay, sustain, and release for all the warp sample voices
    pub fn set_adsr_warp(&mut self, attack_: f32, decay_: f32, sustain_: f32, release_: f32){
        for voice in self.warp_voices.iter_mut(){
            voice.set_adsr(attack_, decay_, sustain_, release_);
        }
    }
    /// Sets the attack, decay, sustain, and release for the given assigned note
    pub fn set_adsr_assign(&mut self, attack_: f32, decay_: f32, sustain_: f32, release_: f32, note_of_assigned: u8){
        if let Some((_file_name, _sr_scalar, _buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            voice.set_adsr(attack_,decay_,sustain_,release_);
        } else {
            // Entry does not exist, handle the error (e.g., log an error message)
            eprintln!("Entry for note {} does not exist in sound bank", note_of_assigned);
        }
    }
    /// Returns attack, decay, sustain, release values for the warping sampler
    /// 
    /// Returns tuple in format: (attack,decay,sustain,release)
    pub fn get_adsr_warp(&mut self)->(f32, f32, f32, f32){
        self.warp_voices[0].adsr.get_adsr()
    }
    /// Returns attack, decay, sustain, release values for the given assigned note
    /// 
    /// Returns tuple in format: (attack,decay,sustain,release)
    pub fn get_adsr_assign(&mut self, note_of_assigned: u8)->(f32, f32, f32, f32){
        if let Some((_file_name, _sr_scalar, _buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            voice.adsr.get_adsr()
        } else {
            (0.1,0.1,1.0,0.1)// Returns default if note not found in map
        }
    }
    /// Sets the max number of voices in the warp sampler
    pub fn set_num_voices(&mut self, mut num_voices: u8){
        if num_voices < 1{
            num_voices = 1;
        }else if num_voices > 24{
            num_voices = 24;
        }
        self.num_voices = num_voices;
        self.warp_voices.resize(num_voices as usize, 
            SamplerVoice::new(self.num_channels,self.sample_rate, 64,VoiceType::Warp));
    }
    /// Returns the number of voices available for the warping sampler
    pub fn get_num_voices(&mut self)->u8{
        self.num_voices
    }
    /// Sets the sampler mode (Warp, Assign, Sfz)
    pub fn set_mode(&mut self, mode: SamplerMode){
        self.sampler_mode = mode;
    }
    /// Sets the note for the warping to be based on
    pub fn set_warp_base(&mut self, base_note: u8){
        for voice in self.warp_voices.iter_mut(){
            match self.sampler_mode {
                SamplerMode::Warp => {voice.set_base_midi(base_note);},
                SamplerMode::Assign => {},
                SamplerMode::Sfz => {}
            }
        }
    }
    /// Returns the internal buffer for the warping sampler for use in the gui
    pub fn get_warp_buffer(& self)-> RingBuffer<f32>{
        self.warp_buffer.clone()
    }
    /// Returns the buffer for the sample assigned to the given note
    pub fn get_assign_buffer(&mut self, note_of_assigned: u8 )->RingBuffer<f32>{
        if let Some((_file_name, _sr_scalar, buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            // Entry exists, update the points
            buff.clone()
        } else {
            RingBuffer::<f32>::new(1)
        }
    }
    /// Sets the start and end points for each of the voices for the warping sampler
    /// 
    /// start_point: (0%-100%),     end_point: (0%-100%)
    ///  
    /// If the start point is greater than the endpoint, the playback will be reversed
    pub fn set_points_warp(&mut self, start_point: f32, end_point: f32){
        for voice in self.warp_voices.iter_mut(){
            match self.sampler_mode {
                SamplerMode::Warp => {voice.set_start_and_end_point(start_point, end_point, self.warp_buffer.capacity());},
                SamplerMode::Assign => {},
                SamplerMode::Sfz => {voice.set_start_and_end_point(start_point, end_point, voice.internal_buffer.capacity());}
            }
        }
    }
    /// Gets the start and end points (in percent) for the warp sampler
    /// 
    ///  Returns tuple in the format: (start_point, end_point)
    pub fn get_points_warp(&mut self)->(f32,f32){
        match self.sampler_mode {
            SamplerMode::Warp => {self.warp_voices[0].get_points(self.warp_buffer.capacity())},
            SamplerMode::Assign => {(0.0,0.0)},
            SamplerMode::Sfz => {(0.0,0.0)}
        }
    }
    /// Sets the start and end points for an assigned sampler voice
    /// 
    /// 
    /// start_point: (0%-100%),     end_point: (0%-100%), note_of_assignment: note of the 
    ///  
    /// If the start point is greater than the endpoint, the playback will be reversed
    pub fn set_points_assign(&mut self, start_point: f32, end_point: f32, note_of_assigned: u8) {
        // Attempt to retrieve the entry corresponding to the given note_of_assigned
        if let Some((_file_name, _sr_scalar, buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            // Entry exists, update the points
            voice.set_start_and_end_point(start_point, end_point, buff.capacity());
        } else {
            // Entry does not exist, handle the error (e.g., log an error message)
            eprintln!("Entry for note {} does not exist in sound bank", note_of_assigned);
        }
    }
    /// Gets the start and end points (in percent) of the voice assigned to the given midi note
    /// 
    /// Returns tuple in the format: (start_point, end_point)
    pub fn get_points_assign(&mut self, note_of_assigned: u8)->(f32,f32){
        if let Some((_file_name, _sr_scalar, buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            // Entry exists, update the points
            voice.get_points(buff.capacity())
        } else{
            (0.0,100.0)// Return defaults if note not found
        }
    }
    /// Sets the start and end points of the warp buffer's sustain looping. Values will be clamped
    /// within start and end points of the sample as a whole
    pub fn set_sus_points_warp(&mut self, start_point: f32, end_point: f32){
        for voice in self.warp_voices.iter_mut(){
            match self.sampler_mode {
                SamplerMode::Warp => {voice.set_sus_points(start_point, end_point, self.warp_buffer.capacity());},
                SamplerMode::Assign => {},
                SamplerMode::Sfz => {voice.set_sus_points(start_point, end_point, voice.internal_buffer.capacity());}
            }
        }
    }
    /// Gets the start and end points for the sustain loop of the warp sampler.
    /// 
    /// Returns tuple in the format: (start_point, end_point)
    pub fn get_sus_points_warp(&mut self)->(f32,f32){
        match self.sampler_mode {
            SamplerMode::Warp => {self.warp_voices[0].get_sus_points(self.warp_buffer.capacity())},
            SamplerMode::Assign => {(0.0,0.0)},
            SamplerMode::Sfz => {(0.0,0.0)}
        }
    }
    /// Sets the start and end points of the assigned buffer's sustain looping. Values will be clamped
    /// within start and end points of the sample as a whole
    pub fn set_sus_points_assign(&mut self, start_point: f32, end_point: f32, note_of_assigned: u8){
        if let Some((_file_name, _sr_scalar, buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            // Entry exists, update the points
            voice.set_sus_points(start_point, end_point, buff.capacity());
        } else {
            // Entry does not exist, handle the error (e.g., log an error message)
            eprintln!("Entry for note {} does not exist in sound bank", note_of_assigned);
        }
    }
    /// Gets the start and end points for the sustain loop of the assigned note.
    /// 
    /// Returns tuple in the format: (start_point, end_point)
    pub fn get_sus_points_assign(&mut self, note_of_assigned: u8)->(f32,f32){
        if let Some((_file_name, _sr_scalar, buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            // Entry exists, update the points
            voice.get_sus_points(buff.capacity())
        } else{
            (0.0,100.0)// Return defaults if note not found
        }
    }
    /// Sets the sustain looping mode for the warping sampler
    pub fn set_sus_looping_warp(&mut self, mode: SustainModes){
        for voice in self.warp_voices.iter_mut(){
            voice.set_sus_loop_mode(mode);
        }
    }
    /// Sets the sustain looping mode for the assign sampler
    pub fn set_sus_looping_assign(&mut self, mode: SustainModes, note_of_assigned: u8){
        if let Some((_file_name, _sr_scalar, _buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            // Entry exists, update the points
            voice.set_sus_loop_mode(mode);
        } else {
            // Entry does not exist, handle the error (e.g., log an error message)
            eprintln!("Entry for note {} does not exist in sound bank", note_of_assigned);
        }
    }
    /// Sets crossfade time in seconds for the warp sampler, expects values between (0.00001 and 0.1)
    pub fn set_fade_time_warp(&mut self, fade_time: f32){
        for voice in self.warp_voices.iter_mut(){
            voice.set_fade_time(fade_time);
        }
    }
    /// Sets crossfade time in seconds for the selected file, expects values between (0.00001 and 0.1)
    pub fn set_fade_time_assign(&mut self, fade_time: f32, note_of_assigned: u8){
        if let Some((_file_name, _sr_scalar, _buff, voice)) = self.sound_bank.get_mut(&note_of_assigned) {
            // Entry exists, update the points
            voice.set_fade_time(fade_time);
        } else {
            // Entry does not exist, handle the error (e.g., log an error message)
            eprintln!("Entry for note {} does not exist in sound bank", note_of_assigned);
        }
    }
    /// Chooses a voice and steals the quietest one
    fn get_voice_id(&mut self)-> usize{
        for (voice_id, voice) in self.warp_voices.iter_mut().enumerate() {
            if !voice.is_active() {
                return voice_id;
            }
        }
        if let Some((quietest_voice_id, _)) = self
            .warp_voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.adsr.state == AdsrState::Release)
            .min_by(|(_, voice_a), (_, voice_b)| {
                f32::total_cmp(
                    &voice_a.adsr.envelope_value,
                    &voice_b.adsr.envelope_value,
                )
            })
        {
            return quietest_voice_id;
        }

        let (quietest_voice_id, _) = self
            .warp_voices
            .iter()
            .enumerate()
            .min_by(|(_, voice_a), (_, voice_b)| {
                f32::total_cmp(
                    &voice_a.adsr.envelope_value,
                    &voice_b.adsr.envelope_value,
                )
            })
            .unwrap();

        quietest_voice_id
    }

}

/// Fills a buffer with a file from a path
fn fill_warp_buffer(buffer: &mut RingBuffer<f32>, path: &str) ->f32{
    if let Ok(mut reader) = hound::WavReader::open(path){
        let sample_format = reader.spec().sample_format;
    let sample_rate = reader.spec().sample_rate as f32;
    let length = reader.len();
    buffer.resize(length as usize, 0.0);
    buffer.set_write_index(0);
    // Determine the conversion factor based on sample format
    let conversion_factor = match sample_format {
        SampleFormat::Float => 1.0, // No conversion needed
        SampleFormat::Int => {
            match reader.spec().bits_per_sample {
                8 => 1.0 / (i8::MAX as f32),
                16 => 1.0 / (i16::MAX as f32),
                24 => 1.0 / (8388608 as f32), 
                _ => panic!("Unsupported bit depth"),
            }
        }
    };
    match sample_format{
        SampleFormat::Float => {
            let mut samples = reader.samples::<f32>();
            for _ in 0..(length) {
                if let Some(sample) = samples.next() {
                    if let Ok(sample_value) = sample {
                        let sample_float = sample_value * conversion_factor;
                         buffer.push(sample_float);
                    }
                }
            }
        }, 
        SampleFormat::Int => {
            let mut samples = reader.samples::<i32>();
            for _ in 0..(length) {
                
                if let Some(sample) = samples.next() {
                    if let Ok(sample_value) = sample {
                        let sample_float = (sample_value as f32) * conversion_factor;
                        buffer.push(sample_float);
                    }
                }
            }
        }
    }
    sample_rate as f32
    }else {
        dbg!("File Failed to load");
        44100.0
    }
    
}

fn create_buffer(path: &str)-> (RingBuffer<f32>,f32){
    if let Ok(mut reader) = hound::WavReader::open(path){
        let sample_format = reader.spec().sample_format;
    let num_channels = reader.spec().channels as usize;
    let sample_rate = reader.spec().sample_rate as f32;
    let length = reader.len();
    let mut buffer = RingBuffer::<f32>::new(length as usize);
    buffer.set_write_index(0);
    // Determine the conversion factor based on sample format
    let conversion_factor = match sample_format {
        SampleFormat::Float => 1.0, // No conversion needed
        SampleFormat::Int => {
            match reader.spec().bits_per_sample {
                8 => 1.0 / (i8::MAX as f32),
                16 => 1.0 / (i16::MAX as f32),
                24 => 1.0 / (8388608 as f32), 
                _ => panic!("Unsupported bit depth"),
            }
        }
    };
    match sample_format{
        SampleFormat::Float => {
            let mut samples = reader.samples::<f32>();
            for _ in 0..(length) {
                if let Some(sample) = samples.next() {
                    if let Ok(sample_value) = sample {
                        let sample_float = sample_value * conversion_factor;
                         buffer.push(sample_float);
                    }
                }
            }
        }, 
        SampleFormat::Int => {
            let mut samples = reader.samples::<i32>();
            for _ in 0..(length) {
                
                if let Some(sample) = samples.next() {
                    if let Ok(sample_value) = sample {
                        let sample_float = (sample_value as f32) * conversion_factor;
                        buffer.push(sample_float);
                    }
                }
            }
        }
    }
    (buffer,sample_rate)
    }else{
        dbg!("File Failed to load");
        (RingBuffer::new(1), 44100.0)
    }
    
}