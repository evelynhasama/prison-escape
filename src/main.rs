struct World {
    maps:Vec<Map>,
    current_map:usize, // An index into the maps vec.
}
struct Map {
    // There are other ways to do this, but this is fine for now.
    tiles:[[Tile; 80]; 23], // An grid of 23 rows of 80 Tiles
    entities:Vec<Thing>, // We'll assume the player is always the 0th thing in this list
    // There's no strong reason the player
    // has to be "just another entity"---we could just as well
    // have separate vecs of each type of entity, and a
    // special `player:Option<Player>` field, or make the Player a property
    // of the world and not the room.  This is just one
    // arbitrary choice among many possible alternatives.
}
#[derive(Clone,Copy)]
enum Tile {
    Empty,
    Wall,
    Stairs(usize,(u8,u8)), // The usize here is the map to which the stairs go, the tuple is where in that map
}
struct Thing {
    position:(u8,u8),
    thing_type:ThingType
}
enum ThingType {
    // These variants are empty now but they could just as well be given associated data
    Player/* (PlayerData) */,
    Enemy/* (EnemyType, EnemyData) */,
    // Treasure,
    // ...
}

enum GameMode {
    Playing,
    InventoryMenu,
    ShopMenu
}

use crossterm::style::{Color, Colors};
trait Style {
    fn colors(&self) -> Colors;
    fn look(&self) -> char;
}
impl Style for Tile {
    fn colors(&self) -> Colors {
        match self {
            Tile::Empty => Colors{foreground:Some(Color::Black), background:Some(Color::Black)},
            Tile::Wall => Colors{foreground:Some(Color::White), background:Some(Color::Black)},
            Tile::Stairs(_,_) => Colors{foreground:Some(Color::White), background:Some(Color::Black)},
        }
    }
    fn look(&self) -> char {
        match self {
            Tile::Empty => '.',
            Tile::Wall => '#',
            Tile::Stairs(_,_) => '>',
        }
    }
}
impl Style for ThingType {
    fn colors(&self) -> Colors {
        match self {
            ThingType::Player => Colors{foreground:Some(Color::White), background:Some(Color::Black)},
            ThingType::Enemy => Colors{foreground:Some(Color::Red), background:Some(Color::Black)},
        }
    }
    fn look(&self) -> char {
        match self {
            ThingType::Player => '@',
            ThingType::Enemy => 'E',
        }
    }
}

impl Map {
    fn draw(&self, out:&mut std::io::Stdout) -> std::io::Result<()> {
        // We can scope a use just to a single function, which is nice
        use std::io::Write;
        use crossterm::{terminal, QueueableCommand, cursor, style::{SetColors, Print}};
        out.queue(terminal::BeginSynchronizedUpdate)?;
        for (y,row) in self.tiles.iter().enumerate() {
            out.queue(cursor::MoveTo(0,y as u16))?;
            for tile in row.iter() {
                out.queue(SetColors(tile.colors()))?;
                out.queue(Print(tile.look()))?;
            }
        }
        for ent in self.entities.iter() {
            let (x,y) = ent.position;
            out.queue(cursor::MoveTo(x as u16,y as u16))?;
            out.queue(SetColors(ent.thing_type.colors()))?;
            out.queue(Print(ent.thing_type.look()))?;
        }
        out.queue(crossterm::terminal::EndSynchronizedUpdate)?;
        out.flush()?;
        Ok(())
    }

    fn move_entity(&mut self, which: usize, dx: i8, dy: i8) -> bool {
        let (x, y) = self.entities[which].position;
        let to_x = x as i16 + dx as i16;
        let to_y = y as i16 + dy as i16;
        if !(0_i16..80).contains(&to_x) || !(0_i16..23).contains(&to_y) {
            return false;
        }
        if let Tile::Wall = self.tiles[to_y as usize][to_x as usize] {
            return false;
        }
        self.entities[which].position = (to_x as u8, to_y as u8);
        true
    }
}

fn parse_tilemap<const W:usize, const H:usize>(text:&'static str) -> [[Tile; W] ; H] {
    let mut ret = [[Tile::Empty; W]; H];
    let chars:Vec<_> = text.chars().collect();
    for (y,row) in chars.chunks(W).enumerate() {
        for (x,ch) in row.iter().enumerate() {
            let tile = match ch {
                '#' => Tile::Wall,
                '.' => Tile::Empty,
                '0'..='9' => Tile::Stairs(
                    ch.to_digit(10).unwrap() as usize,
                    (x as u8,y as u8)
                ),
                _ => Tile::Empty
            };
            ret[y][x] = tile;
        }
    }
    ret
}



fn main() -> std::io::Result<()> {
    use std::io::stdout;
    use crossterm::event::{read, Event, KeyEvent, KeyEventKind, KeyCode};
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let mut stdout = stdout();
    {
        // we can even scope a use to just a single block!
        use crossterm::{terminal, ExecutableCommand};
        terminal::enable_raw_mode()?;
        stdout.execute(crossterm::terminal::SetSize(80,24))?;
        stdout.execute(crossterm::cursor::Hide)?;
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    }

    let mut world = World {
        maps:vec![
            Map {
                tiles:parse_tilemap(include_str!("map0.txt")),
                entities:vec![
                    Thing{thing_type:ThingType::Player, position:(2,2)},
                    Thing{thing_type:ThingType::Enemy, position:(39,17)},
                ]
            },
            Map {
                tiles:parse_tilemap(include_str!("map1.txt")),
                entities:vec![
                    Thing{thing_type:ThingType::Enemy, position:(52,21)},
                ]
            }
        ],
        current_map:0
    };
    let mut game_over = false;
    // One initial draw so that we have something on screen before the first event arrives.
    world.maps[world.current_map].draw(&mut stdout)?;

    // ... event loop and everything else goes here...
    // Get the next event from crossterm, waiting until it's ready
    while let Ok(evt) = read() {
        if let Event::Key(KeyEvent{code,kind:KeyEventKind::Press,..}) = evt {
            if code == KeyCode::Esc {
                break;
            }
            if game_over { continue; }
            let mut status_message = (
                "                                                                                ",
                Colors{foreground:None, background:None}
            );
            // Game rule updates: first, interpret key events.
            // If you have custom game rules you might want e.g. i to open the inventory.
            let (dx,dy) = match code {
                KeyCode::Left => (-1, 0),
                KeyCode::Right => (1, 0),
                KeyCode::Up => (0, -1),
                KeyCode::Down => (0, 1),
                _ => (0,0)
            };
            // Get the current map from the world
            let map = &mut world.maps[world.current_map];
            // Ask it to move our player.  We'll read through this function's code later.
            map.move_entity(0, dx, dy);
            // Then loop through all the other entities and have them move randomly
            for ent in 1..map.entities.len() {
                let dx:i8 = rng.gen_range(-1..=1);
                let dy:i8 = rng.gen_range(-1..=1);
                map.move_entity(ent, dx, dy);
            }
            // Remember where the player is now...
            let (x,y) = map.entities[0].position;
            // if any enemy is touching the player, game over
            for ent in map.entities[1..].iter() {
                // Matching with `if let` is used here since we haven't
                // implemented or derived PartialEq or Eq on ThingType.
                // We'll talk about that another time.
                if let ThingType::Enemy = ent.thing_type {
                    if ent.position == (x,y) {
                        // Set a status message to render later
                        status_message = ("You died!", Colors{foreground:Some(Color::Red), background:Some(Color::Black)});
                        game_over = true;
                    }
                }
            }
            // Maybe move between floors
            if let Tile::Stairs(to_map, to_pos) = map.tiles[y as usize][x as usize] {
                world.current_map = to_map;
                // We'll also move the special player entity where it goes
                // in the new room.
                let mut player = map.entities.remove(0);
                player.position = to_pos;
                world.maps[to_map].entities.insert(0,player);
            }
            // Update's done; render the game state.
            world.maps[world.current_map].draw(&mut stdout)?;
            {
                use crossterm::ExecutableCommand;
                stdout.execute(crossterm::cursor::MoveTo(0, 23))?;
                stdout.execute(crossterm::style::SetColors(status_message.1))?;
                stdout.execute(crossterm::style::Print(status_message.0))?;
            }
        }
    }

    // Then we finally clean up:
    {
        use crossterm::{terminal,ExecutableCommand};
        terminal::disable_raw_mode()?;
        stdout.execute(crossterm::cursor::Show)?;
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    }
    Ok(())
}
