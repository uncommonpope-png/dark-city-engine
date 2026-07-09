pub use bevy_ecs::component::Component as DarkCityComponent;
pub use bevy_ecs::entity::Entity as DarkCityEntity;
pub use bevy_ecs::world::World as DarkCityWorld;
use web_time::Instant;

pub mod ecs;
pub mod renderer;
pub mod shader;
pub mod camera;
pub mod asset;
pub mod input;

use ecs::DarkCityWorldWrapper;
use renderer::WgpuRenderer;
use camera::Camera;

pub struct Time {
    pub delta: f32,
    pub elapsed: f32,
    last_tick: Instant,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta: 0.0,
            elapsed: 0.0,
            last_tick: Instant::now(),
        }
    }
}

pub trait DarkCitySystem {
    fn run(&mut self, world: &mut DarkCityWorldWrapper, time: &Time);
}

pub struct Game {
    world: DarkCityWorldWrapper,
    renderer: WgpuRenderer,
    systems: Vec<Box<dyn DarkCitySystem>>,
    time: Time,
}

impl Game {
    pub async fn new(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
        let renderer = WgpuRenderer::new(canvas).await?;
        Ok(Self {
            world: DarkCityWorldWrapper::new(),
            renderer,
            systems: Vec::new(),
            time: Time::default(),
        })
    }

    pub fn new_for_tests() -> Self {
        Self {
            world: DarkCityWorldWrapper::new(),
            renderer: WgpuRenderer::default(),
            systems: Vec::new(),
            time: Time::default(),
        }
    }

    pub fn add_system<S: DarkCitySystem + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }

    pub fn tick(&mut self) -> Result<(), String> {
        let now = Instant::now();
        self.time.delta = now.duration_since(self.time.last_tick).as_secs_f32();
        self.time.elapsed += self.time.delta;
        self.time.last_tick = now;

        for system in &mut self.systems {
            system.run(&mut self.world, &self.time);
        }
        self.renderer.render()
    }

    pub fn world(&self) -> &DarkCityWorldWrapper {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut DarkCityWorldWrapper {
        &mut self.world
    }

    pub fn time(&self) -> &Time {
        &self.time
    }

    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer {
        &mut self.renderer
    }
}

pub struct LoggingSystem;
impl DarkCitySystem for LoggingSystem {
    fn run(&mut self, _world: &mut DarkCityWorldWrapper, _time: &Time) {
        println!("Tick triggered");
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position(pub f32, pub f32);

impl DarkCityComponent for Position {
    const STORAGE_TYPE: bevy_ecs::component::StorageType = bevy_ecs::component::StorageType::Table;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_entities() {
        let mut game = Game::new_for_tests();
        let entity = game.world_mut().spawn_entity();

        assert_eq!(entity.index(), 0);
    }

    #[test]
    fn adds_and_retrieves_position_component() {
        let mut game = Game::new_for_tests();
        let entity = game.world_mut().spawn_entity();
        game.world_mut().add_component(entity, Position(3.0, 4.0));

        let position = game.world().get_component::<Position>(entity);
        assert_eq!(position, Some(&Position(3.0, 4.0)));
    }

    #[test]
    fn system_executes_in_tick() {
        let mut game = Game::new_for_tests();
        game.add_system(LoggingSystem);
        game.tick().unwrap();
    }
}