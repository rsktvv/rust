use std::rc::Rc;
use std::cell::RefCell;
use std::io::{self, Write};

pub type SharedEntity = Rc<RefCell<Entity>>;

// --- DAMAGE TYPES ---
#[derive(Debug, Clone, Copy)]
pub enum DamageKind {
    Physical,
    Magical,
    Pure,
}

impl std::fmt::Display for DamageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DamageKind::Physical => write!(f, "Physical"),
            DamageKind::Magical => write!(f, "Magical"),
            DamageKind::Pure => write!(f, "Pure"),
        }
    }
}

// --- EFFECTS ---
pub trait Effect: std::fmt::Debug {
    fn apply(&mut self, entity: &mut Entity) -> bool;
    fn name(&self) -> &str;
}

#[derive(Debug)]
pub struct Poison {
    pub name: String,
    pub damage_per_turn: i32,
    pub remaining_turns: usize,
}

impl Poison {
    pub fn new(damage_per_turn: i32, duration: usize) -> Self {
        Self {
            name: "Poison".to_string(),
            damage_per_turn,
            remaining_turns: duration,
        }
    }
}

impl Effect for Poison {
    fn apply(&mut self, entity: &mut Entity) -> bool {
        entity.hp -= self.damage_per_turn;
        self.remaining_turns -= 1;
        self.remaining_turns > 0
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// --- ENTITY ---
#[derive(Debug)]
pub struct Entity {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub mana: i32,
    pub strength: i32,
    pub effects: Vec<Box<dyn Effect>>,
    pub potions: i32,
}

impl Entity {
    pub fn peasant(name: String) -> SharedEntity {
        Rc::new(RefCell::new(Entity {
            name,
            hp: 30,
            max_hp: 30,
            mana: 5,
            strength: 5,
            effects: Vec::new(),
            potions: 0,
        }))
    }

    pub fn dragon(name: String) -> SharedEntity {
        Rc::new(RefCell::new(Entity {
            name,
            hp: 120,
            max_hp: 120,
            mana: 30,
            strength: 25,
            effects: Vec::new(),
            potions: 0,
        }))
    }

    pub fn demon(name: String) -> SharedEntity {
        Rc::new(RefCell::new(Entity {
            name,
            hp: 100,
            max_hp: 100,
            mana: 40,
            strength: 20,
            effects: Vec::new(),
            potions: 0,
        }))
    }

    pub fn lord(name: String) -> SharedEntity {
        Rc::new(RefCell::new(Entity {
            name,
            hp: 200,
            max_hp: 200,
            mana: 100,
            strength: 35,
            effects: Vec::new(),
            potions: 0,
        }))
    }

    pub fn hero() -> SharedEntity {
        Rc::new(RefCell::new(Entity {
            name: "Hero".to_string(),
            hp: 150,
            max_hp: 150,
            mana: 70,
            strength: 18,
            effects: Vec::new(),
            potions: 3,
        }))
    }

    pub fn apply_damage(&mut self, kind: DamageKind, mut amount: i32) {
        let resist = self.get_resistance(kind);
        amount = (amount as f32 * resist) as i32;

        if amount > 0 {
            self.hp -= amount;
            println!("{} takes {} {} damage!", self.name, amount, kind);
        } else {
            println!("{} takes no damage from this type.", self.name);
        }
    }

    fn get_resistance(&self, kind: DamageKind) -> f32 {
        use DamageKind::*;

        match self.name.as_str() {
            "Peasant" | "Dragon" => match kind {
                Physical => 1.0,
                Magical => 1.0,
                Pure => 0.0,
            },

            "Demon" => match kind {
                Physical => 0.0,
                Magical => 0.5,
                Pure => 1.0,
            },

            "Lord" => match kind {
                Physical | Magical => 0.0,
                Pure => 3.0,
            },

            _ => 1.0,
        }
    }

    pub fn heal(&mut self, amount: i32) {
        let new_hp = (self.hp + amount).min(self.max_hp);
        let actual = new_hp - self.hp;
        self.hp = new_hp;
        if actual > 0 {
            println!("{} heals for {} HP!", self.name, actual);
        }
    }

    pub fn use_potion(&mut self) -> bool {
        if self.potions > 0 {
            self.potions -= 1;
            self.heal(50);
            true
        } else {
            println!("No potions left!");
            false
        }
    }

    pub fn next_turn(&mut self) {
        // Перепишем без захвата `self` в замыкании
        let mut effects = std::mem::take(&mut self.effects);
        effects.retain_mut(|effect| effect.apply(self));
        self.effects = effects;
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

// --- COMMANDS ---
pub trait Command {
    fn execute(&self);
}

#[derive(Debug)]
pub struct PhysicalAttackCommand {
    pub attacker: SharedEntity,
    pub target: SharedEntity,
}

impl Command for PhysicalAttackCommand {
    fn execute(&self) {
        let attacker = self.attacker.borrow();
        let target = self.target.borrow();
        let damage = attacker.strength;
        drop(attacker); drop(target);

        let mut target = self.target.borrow_mut();
        target.apply_damage(DamageKind::Physical, damage);
    }
}

#[derive(Debug)]
pub struct FireAttackCommand {
    pub attacker: SharedEntity,
    pub target: SharedEntity,
    pub base_damage: i32,
}

impl Command for FireAttackCommand {
    fn execute(&self) {
        let mut target = self.target.borrow_mut(); // Добавлен mut
        target.apply_damage(DamageKind::Magical, self.base_damage);
    }
}

#[derive(Debug)]
pub struct ScreamAttackCommand {
    pub attacker: SharedEntity,
    pub target: SharedEntity,
    pub base_damage: i32,
}

impl Command for ScreamAttackCommand {
    fn execute(&self) {
        let mut target = self.target.borrow_mut(); // Добавлен mut
        target.apply_damage(DamageKind::Pure, self.base_damage);
    }
}

#[derive(Debug)]
pub struct UsePotionCommand {
    pub hero: SharedEntity,
}

impl Command for UsePotionCommand {
    fn execute(&self) {
        let mut h = self.hero.borrow_mut();
        if h.potions > 0 {
            h.use_potion();
        }
    }
}

// --- ACTION FACTORY: static functions (no `factory` variable) ---
pub struct ActionFactory;

impl ActionFactory {
    pub fn make_sword_attack(
        attacker: SharedEntity,
        target: SharedEntity,
    ) -> Box<dyn Command> {
        Box::new(PhysicalAttackCommand { attacker, target })
    }

    pub fn make_fire_attack(
        attacker: SharedEntity,
        target: SharedEntity,
    ) -> Box<dyn Command> {
        Box::new(FireAttackCommand {
            attacker,
            target,
            base_damage: 20,
        })
    }

    pub fn make_scream_attack(
        attacker: SharedEntity,
        target: SharedEntity,
    ) -> Box<dyn Command> {
        Box::new(ScreamAttackCommand {
            attacker,
            target,
            base_damage: 10,
        })
    }

    pub fn make_phys_attack_or_other(
        attacker: SharedEntity,
        target: SharedEntity,
    ) -> Box<dyn Command> {
        Box::new(PhysicalAttackCommand { attacker, target })
    }

    pub fn make_use_potion(hero: SharedEntity) -> Box<dyn Command> {
        Box::new(UsePotionCommand { hero })
    }
}

// --- BATTLE ENGINE ---
pub struct BattleEngine {
    pub commands: Vec<Box<dyn Command>>,
}

impl BattleEngine {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, cmd: Box<dyn Command>) {
        self.commands.push(cmd);
    }

    pub fn tick(&mut self, entities: &mut Vec<SharedEntity>) {
        while let Some(cmd) = self.commands.pop() {
            cmd.execute();
        }

        for entity in entities {
            let mut e = entity.borrow_mut();
            e.next_turn();
        }
    }

    pub fn clear_commands(&mut self) {
        self.commands.clear();
    }
}

// --- HELPERS ---
fn make_enemies() -> Vec<SharedEntity> {
    vec![
        Entity::peasant("Peasant".to_string()),
        Entity::dragon("Dragon".to_string()),
        Entity::demon("Demon".to_string()),
        Entity::lord("Lord".to_string()),
    ]
}

fn are_all_enemies_dead(enemies: &[SharedEntity]) -> bool {
    enemies.iter().all(|e| !e.borrow().is_alive())
}

fn is_player_dead(player: &SharedEntity) -> bool {
    !player.borrow().is_alive()
}

fn print_battle_status(hero: &SharedEntity, enemies: &[SharedEntity]) {
    let h = hero.borrow();
    println!(
        "{}: HP={}/{}, Mana={}, Potions={}, Effects: {:?}",
        h.name, h.hp, h.max_hp, h.mana, h.potions, h.effects
    );
    drop(h);

    for enemy in enemies {
        let e = enemy.borrow();
        println!(
            "{}: HP={}/{}, Mana={}, Effects: {:?}",
            e.name, e.hp, e.max_hp, e.mana, e.effects
        );
    }
    println!("---");
}

fn find_first_alive_enemy(enemies: &[SharedEntity]) -> Option<SharedEntity> {
    enemies
        .iter()
        .find(|e| e.borrow().is_alive())
        .map(ToOwned::to_owned)
}

fn read_player_choice() -> Result<String, std::io::Error> {
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

// --- MAIN GAME LOOP (with ActionFactory::... syntax) ---
fn main() {
    let hero = Entity::hero();
    let enemies = make_enemies();

    let mut entities = vec![hero.clone()];
    entities.extend(enemies.iter().cloned());

    let mut engine = BattleEngine::new();
    let mut defeated_enemies = 0;

    loop {
        print_battle_status(&hero, &enemies);

        let mut hero_used_action = false;
        while !hero_used_action {
            print!(
                "Choose action: [1] Sword (Phys) [2] Fire (Mag) [3] Scream (Pure) [4] Heal ({} potions) [5] Status [6] Quit: ",
                hero.borrow().potions,
            );

            match read_player_choice() {
                Ok(choice) => match choice.as_str() {
                    "1" => {
                        if let Some(target) = find_first_alive_enemy(&enemies) {
                            let cmd = ActionFactory::make_sword_attack(hero.clone(), target);
                            engine.enqueue(cmd);
                            hero_used_action = true;
                        } else {
                            println!("No enemies left!");
                        }
                    }
                    "2" => {
                        let mana_cost = 15;
                        {
                            let h = hero.borrow();
                            if h.mana >= mana_cost {
                                drop(h); // освобождаем immer
                                let mut h = hero.borrow_mut();
                                h.mana -= mana_cost;

                                if let Some(target) = find_first_alive_enemy(&enemies) {
                                    let cmd = ActionFactory::make_fire_attack(hero.clone(), target);
                                    engine.enqueue(cmd);
                                    hero_used_action = true;
                                }
                            } else {
                                println!("Not enough mana!");
                            }
                        }
                    }
                    "3" => {
                        if let Some(target) = find_first_alive_enemy(&enemies) {
                            let cmd = ActionFactory::make_scream_attack(hero.clone(), target);
                            engine.enqueue(cmd);
                            hero_used_action = true;
                        }
                    }
                    "4" => {
                        let mut h = hero.borrow_mut();
                        if h.potions > 0 {
                            let cmd = ActionFactory::make_use_potion(hero.clone());
                            engine.enqueue(cmd);
                            hero_used_action = true;
                        } else {
                            println!("No potions left! Choose another action.");
                        }
                    }
                    "5" => print_battle_status(&hero, &enemies),
                    "6" => {
                        println!("Bye!");
                        return;
                    }
                    _ => println!("Unknown command."),
                },
                Err(_) => {
                    println!("Error reading input.");
                    return;
                }
            }
        }

        // Враги атакуют героя
        for enemy in &enemies {
            if enemy.borrow().is_alive() {
                let cmd = ActionFactory::make_phys_attack_or_other(enemy.clone(), hero.clone());
                engine.enqueue(cmd);
            }
        }

        engine.tick(&mut entities.iter().map(ToOwned::to_owned).collect());

        // Проверка и добавление хилок
        let mut new_defeated = 0;
        for enemy in &enemies {
            if !enemy.borrow().is_alive() {
                new_defeated += 1;
            }
        }

        if new_defeated > defeated_enemies {
            let mut h = hero.borrow_mut();
            h.potions += 1;
            println!("Hero gains 1 potion! Potions: {}", h.potions);
        }
        defeated_enemies = new_defeated;

        if is_player_dead(&hero) {
            println!("You died! Game over.");
            break;
        }

        if are_all_enemies_dead(&enemies) {
            println!("You defeated all enemies: Peasant, Dragon, Demon and Lord!");
            println!("You won the game!");
            break;
        }
    }
}
