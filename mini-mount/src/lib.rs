pub struct Mini {
    address: u8,
    clock_state: u8,
}

impl Mini {
    pub fn new(address: u8) -> Self {
        Self {
            address,
            clock_state: 0,
        }
    }

    pub fn synchronize(&mut self) {
        self.clock_state = 0;
    }

    pub fn tick(&mut self) -> bool {
        self.clock_state = self.clock_state.saturating_add(1);
        self.clock_state == self.address
    }
}
