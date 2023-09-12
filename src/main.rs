struct World {
    maps:Vec<Map>,
    current_map:usize, // An index into the maps vec.
}
struct Map {
    tiles:[[Tile; 81]; 23], // An grid of 23 rows of 80 Tiles
    entities:Vec<Thing>, // We'll assume the player is always the 0th thing in this list
}
#[derive(Clone,Copy)]
enum Tile {
    Empty,
    Wall,
    Key(DoorID), 
    Door(DoorID), 
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct DoorID(usize);
struct Thing {
    position:(u8,u8),
    thing_type:ThingType
}
enum ThingType {
    Prisoner,
    Guard,
}

struct PrisonerState {
    keys: Vec<DoorID>,
    health: usize,
}

use crossterm::{style::{Color, Colors}, ExecutableCommand};
trait Style {
    fn colors(&self) -> Colors;
    fn look(&self) -> char;
}
impl Style for Tile {
    fn colors(&self) -> Colors {
        match self {
            Tile::Empty => Colors{foreground:Some(Color::Black), background:Some(Color::Black)},
            Tile::Wall => Colors{foreground:Some(Color::White), background:Some(Color::Black)},
            Tile::Door(_) => Colors{foreground:Some(Color::Cyan), background:Some(Color::Black)},
            Tile::Key(_) => Colors{foreground:Some(Color::Yellow), background:Some(Color::Black)},
        }
    }
    fn look(&self) -> char {
        match self {
            Tile::Empty => '.',
            Tile::Wall => '#',
            Tile::Door(_) => '>',
            Tile::Key(_) => '*',
        }
    }
}
impl Style for ThingType {
    fn colors(&self) -> Colors {
        match self {
            ThingType::Prisoner => Colors{foreground:Some(Color::Green), background:Some(Color::Black)},
            ThingType::Guard => Colors{foreground:Some(Color::Red), background:Some(Color::Black)},
        }
    }
    fn look(&self) -> char {
        match self {
            ThingType::Prisoner => '@',
            ThingType::Guard => 'G',
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
                '0'..='4' => Tile::Door(
                    DoorID(ch.to_digit(10).unwrap() as usize),
                ),
                '5'..='9' => Tile::Key(
                    DoorID(ch.to_digit(10).unwrap() as usize - 5)
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
    use crossterm::terminal;

    let mut rng = rand::thread_rng();
    let mut stdout = stdout();
    {
        terminal::enable_raw_mode()?;
        stdout.execute(crossterm::terminal::SetSize(80,27))?;
        stdout.execute(crossterm::cursor::Hide)?;
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    }

    let mut world = World {
        maps:vec![
            Map {
                tiles:parse_tilemap(include_str!("map0.txt")),
                entities:vec![
                    Thing{thing_type:ThingType::Prisoner, position:(53,7)},
                    Thing{thing_type:ThingType::Guard, position:(21,15)},
                    Thing{thing_type:ThingType::Guard, position:(65,11)},
                    Thing{thing_type:ThingType::Guard, position:(74,20)},
                    Thing{thing_type:ThingType::Guard, position:(8,19)},
                ]
            },
            Map {
                tiles:parse_tilemap(include_str!("map1.txt")),
                entities:vec![
                    Thing{thing_type:ThingType::Guard, position:(7,9)},
                    Thing{thing_type:ThingType::Guard, position:(76,14)},
                    Thing{thing_type:ThingType::Guard, position:(39,15)},
                ]
            },
            Map {
                tiles:parse_tilemap(include_str!("map2.txt")),
                entities:vec![
                    Thing{thing_type:ThingType::Guard, position:(47,9)},
                    Thing{thing_type:ThingType::Guard, position:(69,13)},
                    Thing{thing_type:ThingType::Guard, position:(19,22)},
                    Thing{thing_type:ThingType::Guard, position:(29,3)},
                ]
            }
        ],
        current_map:0
    };
    let mut game_over = false;
    let mut prisoner_state: PrisonerState = PrisonerState{keys: [].to_vec(), health: 100};
    // One initial draw so that we have something on screen before the first event arrives.
    world.maps[world.current_map].draw(&mut stdout)?;

    // print instructions
    stdout.execute(crossterm::cursor::MoveTo(0, 23))?;
    stdout.execute(crossterm::style::SetColors(Colors{foreground:Some(Color::Black), background:Some(Color::White)}))?;
    let instruction1 = "Escape the Prison! ";
    stdout.execute(crossterm::style::Print(instruction1))?;
    stdout.execute(crossterm::cursor::MoveTo(0, 24))?;
    stdout.execute(crossterm::style::SetColors(Colors{foreground:Some(Color::Black), background:Some(Color::White)}))?;
    let instruction1 = "Collect keys * to open doors > Don't get caught by the guards G!";
    stdout.execute(crossterm::style::Print(instruction1))?;
    stdout.execute(crossterm::cursor::MoveTo(0, 25))?;
    stdout.execute(crossterm::style::SetColors(Colors{foreground:Some(Color::Black), background:Some(Color::White)}))?;
    let instruction1 = "You are currently in your cell. Goodluck!";
    stdout.execute(crossterm::style::Print(instruction1))?;

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
                let dx:i8 = rng.gen_range(-2..=2);
                let dy:i8 = rng.gen_range(-2..=2);
                map.move_entity(ent, dx, dy);
            }
            // Remember where the player is now...
            let (x,y) = map.entities[0].position;
            // if any enemy is touching the player, game over
            for ent in map.entities[1..].iter() {
                if let ThingType::Guard = ent.thing_type {
                    if ent.position == (x,y) {
                        prisoner_state.health -= 50;
                        if prisoner_state.health == 0 {
                            // Set a status message to render later
                            status_message = ("You died! Game Over", Colors{foreground:Some(Color::Red), background:Some(Color::White)});
                            game_over = true;
                        } else {
                            status_message = ("A guard hit you", Colors{foreground:Some(Color::Red), background:Some(Color::White)});
                        }
                        
                    }
                }
            }
            // Maybe move between rooms
            if let Tile::Door(door_id       ) = map.tiles[y as usize][x as usize] {
                if door_id == DoorID(3) {
                    status_message = ("You made it out! Enjoy your freedom          ", Colors{foreground:Some(Color::DarkGreen), background:Some(Color::White)});
                    game_over = true;
                }
                else if prisoner_state.keys.contains(&door_id) {
                    world.current_map = world.current_map + 1;
                    // move player to new room
                    let mut player = map.entities.remove(0);
                    if door_id == DoorID(1) {
                        status_message = ("You are entering the infirmary              ", Colors{foreground:Some(Color::Red), background:Some(Color::White)});
                        player.position = (6,0);
                    } else if door_id == DoorID(0) {
                        status_message = ("You are entering the cafeteria              ", Colors{foreground:Some(Color::Red), background:Some(Color::White)});
                        player.position = (74,0);
                    } else {
                        status_message = ("Find your way through the tunnels                   ", Colors{foreground:Some(Color::Red), background:Some(Color::White)});
                        player.position = (40,0);
                    }
                    world.maps[world.current_map].entities.insert(0,player);
                } else {
                    status_message = ("You need a key or the right key to open this door!          ", Colors{foreground:Some(Color::Red), background:Some(Color::White)});
                }
                
            }
            // maybe get a key
            else if let Tile::Key(door_id       ) = map.tiles[y as usize][x as usize] {
                prisoner_state.keys.push(door_id);
                status_message = ("You collected a key!                   ", Colors{foreground:Some(Color::Red), background:Some(Color::White)});
                // remove key from map
                map.tiles[y as usize][x as usize] = Tile::Empty
            }
            // Update's done; render the game state.
            world.maps[world.current_map].draw(&mut stdout)?;
            {
                stdout.execute(crossterm::cursor::MoveTo(0, 23))?;
                stdout.execute(crossterm::style::SetColors(Colors{foreground:Some(Color::Black), background:Some(Color::White)}))?;
                let inventory = "Inventory: ".to_string() + &(prisoner_state.keys.len()).to_string() + " keys                                                               ";
                stdout.execute(crossterm::style::Print(inventory))?;
                stdout.execute(crossterm::cursor::MoveTo(0, 24))?;
                stdout.execute(crossterm::style::SetColors(Colors{foreground:Some(Color::Black), background:Some(Color::White)}))?;
                let inventory = "Health: ".to_string() + &(prisoner_state.health).to_string() + "%                                                                     ";
                stdout.execute(crossterm::style::Print(inventory))?;
                stdout.execute(crossterm::cursor::MoveTo(0, 25))?;
                stdout.execute(crossterm::style::SetColors(status_message.1))?;
                stdout.execute(crossterm::style::Print(status_message.0))?;
            }
        }
    }

    // Then we finally clean up:
    {
        terminal::disable_raw_mode()?;
        stdout.execute(crossterm::cursor::Show)?;
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    }
    Ok(())
}
