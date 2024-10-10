#[derive(Debug, Clone)]
pub struct Turn {
    pub from: i32,
    pub to: i32,
    pub capture: i32,
    pub promotion: i32,
    pub eval: i32,
}

impl Turn {
    // Constructor with all fields
    pub fn new(from: i32, to: i32, capture: i32, promotion: i32, eval: i32) -> Self {
        Turn {
            from,
            to,
            capture,
            promotion,
            eval,
        }
    }

    // Copy constructor
    pub fn from_other(other: &Turn) -> Self {
        Turn {
            from: other.from,
            to: other.to,
            capture: other.capture,
            promotion: other.promotion,
            eval: other.eval,
        }
    }

    // Constructor with only 'from' and 'to' fields
    pub fn from_to(from: i32, to: i32) -> Self {
        Turn {
            from,
            to,
            capture: 0,
            promotion: 0,
            eval: 0,
        }
    }

    // Check if the move is a promotion
    pub fn is_promotion(&self) -> bool {
        self.promotion != 0
    }

    // Set promotion with fluent interface
    pub fn set_promotion(mut self, promotion: i32) -> Self {
        self.promotion = promotion;
        self
    }
}
