use primitive_types::U256;

const MAX_TARGET_HEX: &str = "00000000FFFF0000000000000000000000000000000000000000000000000000";

lazy_static::lazy_static! {
    static ref MAX_TARGET: U256 = U256::from_str_radix(MAX_TARGET_HEX, 16).unwrap();
}

pub struct DifficultyService;

impl DifficultyService {
    pub fn calculate_difficulty_from_hash(target: &[u8]) -> f64 {
        let current_target = U256::from_big_endian(target);
    
        let (numerator, denominator, needs_inversion) = if current_target > *MAX_TARGET {
            (current_target, *MAX_TARGET, true)
        } else {
            (*MAX_TARGET, current_target, false)
        };
    
        let shift_amount = numerator.bits().max(denominator.bits()).saturating_sub(53);
    
        let ratio =
            (numerator >> shift_amount).as_u64() as f64 / (denominator >> shift_amount).as_u64() as f64;
    
        if needs_inversion {
            1.0 / ratio
        } else {
            ratio
        }
    }
}