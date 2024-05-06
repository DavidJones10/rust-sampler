#[derive(Clone)]
pub struct RingBuffer<T> {
    // TODO: fill this in.
    buffer :  Vec<T>,
    read_ptr : usize,
    write_ptr : usize
}

impl<T: Copy + Default> RingBuffer<T> {
    pub fn new(length: usize) -> Self {
        // Create a new RingBuffer with `length` slots and "default" values.
        // Hint: look into `vec!` and the `Default` trait.
        //todo!();
        RingBuffer::<T>{buffer: vec![T::default(); length],
                        read_ptr: 0,
                        write_ptr: 0  }
    }

    pub fn reset(&mut self) {
        // Clear internal buffer and reset indices.
        //todo!()
        for value in self.buffer.iter_mut() {
            *value = T::default();
        }
        self.read_ptr = 0;
        self.write_ptr = 0;
    }

    // `put` and `peek` write/read without advancing the indices.
    pub fn put(&mut self, value: T) {
        //todo!()
        self.buffer[self.write_ptr] = value;
    }

    pub fn peek(&self) -> T {
        //todo!()
        self.buffer[self.read_ptr]
    }

    pub fn get(&self, offset: usize) -> T {
        //todo!()
        let safe_offset = offset % self.buffer.capacity();
        self.buffer.get(safe_offset).copied().unwrap_or_default()
    }

    // `push` and `pop` write/read and advance the indices.
    pub fn push(&mut self, value: T) {
        //todo!()
        self.put(value);
        self.write_ptr = (self.write_ptr + 1) % self.capacity();
    }

    pub fn pop(&mut self) -> T {
        //todo!()
        let val = self.peek();
        self.read_ptr = (self.read_ptr + 1) % self.capacity();
        val

    }

    pub fn get_read_index(&self) -> usize {
        //todo!()
        self.read_ptr
    }

    pub fn set_read_index(&mut self, index: usize) {
        //todo!()
        self.read_ptr = index % self.capacity();
    }

    pub fn get_write_index(&self) -> usize {
        //todo!()
        self.write_ptr
    }

    pub fn set_write_index(&mut self, index: usize) {
        //todo!()
        self.write_ptr = index % self.capacity();
    }

    pub fn len(&self) -> usize {
        // Return number of values currently in the buffer.
        //todo!()
        if self.write_ptr >= self.read_ptr {
            self.write_ptr - self.read_ptr
        } else {
            // Handle the case where write pointer has wrapped around
            self.capacity() - (self.read_ptr - self.write_ptr)
        }
    }

    pub fn capacity(&self) -> usize {
        // Return the length of the internal buffer.
        //todo!()
        self.buffer.len()
    }

    pub fn resize(&mut self, new_size: usize, value: T){
        self.buffer.resize(new_size, value);
    }
}
impl RingBuffer<f32>{
    // returns a value at a a non-integer offset for fractional delays
    pub fn get_frac(&self, offset: f32)->f32{
        if offset == 0.0{
            self.get(0);
        }
        let floor = offset.trunc();
        let floor_samp = self.get(floor as usize);
        let ceil_samp = self.get(floor as usize + 1);
        let frac = offset.fract();
        floor_samp * (1.0 - frac) + ceil_samp * frac
    }
    // meant to be used similarly to pop, simply put in a offset and it will calculate the 
    // read pointer's position based on the write pointer
    pub fn pop_frac(& self, offset: f32)->f32{
        if offset == 0.0{
            self.get(self.write_ptr);
        }
        let fract_ptr_offset = if offset.fract() == 0.0{
            0
        }else{
            1
        };
        let mut read_int = self.write_ptr as i32 - offset.ceil() as i32 - fract_ptr_offset;
        let mut read_point;
        if read_int < 0 {
            read_int += self.capacity() as i32;
            read_point = read_int as usize;
        }else{
            read_point = read_int as usize;
        }
        
        let floor_samp = self.get(read_point);
        let ceil_samp = self.get(read_point + 1_usize);
        let frac = offset.fract();
        floor_samp * (1.0 - frac) + ceil_samp * frac
    }

}