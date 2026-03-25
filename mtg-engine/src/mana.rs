use crate::types::{ManaCost, ManaPool, ManaType, ManaSymbol};
#[cfg(test)]
use crate::types::Color;

#[derive(Debug)]
pub enum ManaError {
    InsufficientMana,
}

/// Check if a mana pool can pay a given cost.
pub fn can_pay(pool: &ManaPool, cost: &ManaCost) -> bool {
    // Clone pool to simulate payment.
    let mut sim = pool.clone();
    try_auto_pay(&mut sim, cost).is_ok()
}

/// Automatically pay a mana cost from a pool.
/// Deducts the mana and returns Ok, or returns Err if insufficient.
///
/// Strategy: pay colored requirements first, then colorless requirements,
/// then generic from whatever remains.
pub fn auto_pay(pool: &mut ManaPool, cost: &ManaCost) -> Result<(), ManaError> {
    try_auto_pay(pool, cost)
}

fn try_auto_pay(pool: &mut ManaPool, cost: &ManaCost) -> Result<(), ManaError> {
    // 1. Pay colored requirements.
    for sym in &cost.symbols {
        if let ManaSymbol::Colored(color) = sym {
            let mana_type = ManaType::from(*color);
            let available = pool.get(mana_type);
            if available == 0 {
                return Err(ManaError::InsufficientMana);
            }
            pool.mana.insert(mana_type, available - 1);
        }
    }

    // 2. Pay specifically colorless requirements.
    let colorless_needed = cost.colorless_amount();
    if colorless_needed > 0 {
        let available = pool.get(ManaType::Colorless);
        if available < colorless_needed {
            return Err(ManaError::InsufficientMana);
        }
        pool.mana.insert(ManaType::Colorless, available - colorless_needed);
    }

    // 3. Pay generic costs from whatever is left.
    let generic_needed = cost.generic_amount();
    if generic_needed > 0 {
        let total_remaining = pool.total();
        if total_remaining < generic_needed {
            return Err(ManaError::InsufficientMana);
        }
        let mut remaining = generic_needed;
        // Pay from colorless first, then from each color.
        let types = [
            ManaType::Colorless,
            ManaType::White, ManaType::Blue, ManaType::Black,
            ManaType::Red, ManaType::Green,
        ];
        for mt in types {
            if remaining == 0 { break; }
            let available = pool.get(mt);
            let to_use = available.min(remaining);
            if to_use > 0 {
                pool.mana.insert(mt, available - to_use);
                remaining -= to_use;
            }
        }
        if remaining > 0 {
            return Err(ManaError::InsufficientMana);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pay_simple_colored() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Green, 2);

        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(can_pay(&pool, &cost));
        assert!(auto_pay(&mut pool, &cost).is_ok());
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn pay_generic_plus_colored() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Red, 2);

        // {1}{R}
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::Colored(Color::Red),
        ]);

        assert!(can_pay(&pool, &cost));
        assert!(auto_pay(&mut pool, &cost).is_ok());
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn insufficient_mana() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Green, 1);

        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(!can_pay(&pool, &cost));
    }

    #[test]
    fn pay_generic_with_mixed_pool() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Red, 1);
        pool.add(ManaType::Green, 2);

        // {2}{G} — should pay G from green, then 2 generic from remaining green + red
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(can_pay(&pool, &cost));
        assert!(auto_pay(&mut pool, &cost).is_ok());
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn wrong_color() {
        let pool_with_red = {
            let mut p = ManaPool::new();
            p.add(ManaType::Red, 2);
            p
        };

        let cost_gg = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(!can_pay(&pool_with_red, &cost_gg));
    }
}
