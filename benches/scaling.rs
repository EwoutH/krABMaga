extern crate abm;

use criterion::*;

use abm::bag_ref::Ref;
use abm::agent::Agent;
use abm::toroidal_transform;
use abm::toroidal_distance;
use abm::Field2D;
use abm::location::Location2D;
use abm::location::Real2D;
use abm::Schedule;
use abm::state::State;
use rand::Rng;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use abm::field::DoubleBufferedField;
use cfg_if::cfg_if;
static mut _COUNT: u128 = 0;
static STEP: usize = 50;
static WIDTH: f64 = 1008.0;
static HEIGTH: f64 = 1008.0;
static DISCRETIZATION: f64 = 10.0 / 1.5;
static TOROIDAL: bool = true;
static COHESION: f64 = 1.0;
static AVOIDANCE: f64 = 1.0;
static RANDOMNESS: f64 = 1.0;
static CONSISTENCY: f64 = 1.0;
static MOMENTUM: f64 = 1.0;
static JUMP: f64 = 0.7;
static WEAK_N:usize = 306000;
static STRONG_N: usize = 3_006_000;
static SUPER_STRONG_N: usize = 12_800_000;
static thread_cfg: [usize;9] = [2,4,8,16,32,36,64,72,128];

cfg_if!{
    if #[cfg(feature ="sequential")]{
        
        fn benchmark_boids(c: &mut Criterion){
            let mut weak_group = c.benchmark_group("Weak Scaling");
            weak_group.sampling_mode(SamplingMode::Flat);
            weak_group.sample_size(10);
            let (mut state,mut schedule) = setup(WEAK_N/128,1);
            weak_group.bench_function(BenchmarkId::new("boids",1),
                |b|{ b.iter(|| simulate(STEP,&mut schedule,&mut state) ) });
            weak_group.finish();

            let mut strong_group = c.benchmark_group("Strong Scaling");
            strong_group.sampling_mode(SamplingMode::Flat);
            strong_group.sample_size(10);
            let (mut state,mut schedule) = setup(STRONG_N/128,1);
            strong_group.bench_function(BenchmarkId::new("boids",1),
                |b|{ b.iter(|| simulate(STEP,&mut schedule,&mut state) ) });
            strong_group.finish();


            let mut super_strong_group = c.benchmark_group("Super Strong Scaling");
            super_strong_group.sampling_mode(SamplingMode::Flat);
            super_strong_group.sample_size(10);
            let (mut state,mut schedule) = setup(SUPER_STRONG_N/128,1);
            super_strong_group.bench_function(BenchmarkId::new("boids",1),
                |b|{ b.iter(|| simulate(STEP,&mut schedule,&mut state) ) });
            super_strong_group.finish();
        }
    }
    else{
        fn benchmark_boids(c: &mut Criterion){
            let mut weak_group = c.benchmark_group("Weak Scaling");
            weak_group.sampling_mode(SamplingMode::Flat);
            weak_group.sample_size(10);
            for n_thread in thread_cfg.iter(){
                let (mut state,mut schedule) = setup(WEAK_N/128 as usize * n_thread,*n_thread);
                weak_group.bench_function(BenchmarkId::new("boids",n_thread),
                    |b|{ b.iter(|| simulate(STEP,&mut schedule,&mut state) ) });
                
               }
            weak_group.finish();

            let mut strong_group = c.benchmark_group("Strong Scaling");
            strong_group.sampling_mode(SamplingMode::Flat);
            strong_group.sample_size(10);
            for n_thread in thread_cfg.iter(){
                let (mut state,mut schedule) = setup(STRONG_N/128 as usize * n_thread,*n_thread);
                strong_group.bench_function(BenchmarkId::new("boids",n_thread),
                    |b|{ b.iter(|| simulate(STEP,&mut schedule,&mut state) ) });
                
            }

            strong_group.finish();
            
            let mut super_strong_group = c.benchmark_group("Super Strong Scaling");
            super_strong_group.sampling_mode(SamplingMode::Flat);
            super_strong_group.sample_size(10);
            for n_thread in thread_cfg.iter(){
                let (mut state,mut schedule) = setup(SUPER_STRONG_N/128 as usize * n_thread,*n_thread);
                super_strong_group.bench_function(BenchmarkId::new("boids",n_thread),
                    |b|{ b.iter(|| simulate(STEP,&mut schedule,&mut state) ) });
            }

            super_strong_group.finish();
        }
    }
}

criterion_group!(benches, benchmark_boids);
criterion_main!(benches);

fn setup(n_agent:usize, n_thread:usize) ->(BoidsState,abm::Schedule<Bird>) {
    let mut rng = rand::thread_rng();
    
    cfg_if!{
        if #[cfg(feature ="sequential")]{
            let mut schedule: Schedule<Bird> = Schedule::new();
        }
        else{
            let mut schedule: Schedule<Bird> = Schedule::with_threads(n_thread);
        }
    }
    // assert!(schedule.events.is_empty());

    let mut state = BoidsState::new(WIDTH, HEIGTH, DISCRETIZATION, TOROIDAL);
    for bird_id in 0..n_agent {
        
        let r1: f64 = rng.gen();
        let r2: f64 = rng.gen();
        let last_d = Real2D { x: 0.0, y: 0.0 };
        let bird = Bird::new(
            bird_id as u128,
            Real2D {
                x: WIDTH * r1,
                y: HEIGTH * r2,
            },
            last_d,
        );
        state
            .field1
            .set_object_location(bird, bird.pos);
    
        schedule.schedule_repeating(bird, 0.0, 0);
    }

    (state,schedule)
}

fn simulate(step:usize,schedule:&mut abm::Schedule<Bird>, state: &mut BoidsState){
    for _ in 0..step {
        schedule.step(state);
    }
}
pub struct BoidsState {
    pub field1: Field2D<Bird>,
}

impl BoidsState {
    pub fn new(w: f64, h: f64, d: f64, t: bool) -> BoidsState {
        BoidsState {
            field1: Field2D::new(w, h, d, t),
        }
    }
}

impl State for BoidsState{
    fn update(&mut self){
        self.field1.update();
    }
}


#[derive(Clone, Copy)]
pub struct Bird {
    pub id: u128,
    pub pos: Real2D,
    pub last_d: Real2D,
}

impl Bird {
    pub fn new(id: u128, pos: Real2D, last_d: Real2D) -> Self {
        Bird { id, pos, last_d }
    }

    pub fn avoidance(self, vec: &Vec<Ref<Bird>>) -> Real2D {
        if vec.is_empty() {
            let real = Real2D { x: 0.0, y: 0.0 };
            return real;
        }

        let mut x = 0.0;
        let mut y = 0.0;

        let mut count = 0;

        for i in 0..vec.len() {
            if self != *vec[i] {
                let dx = toroidal_distance(self.pos.x, vec[i].pos.x, WIDTH);
                let dy = toroidal_distance(self.pos.y, vec[i].pos.y, HEIGTH);
                let square = (dx * dx + dy * dy).sqrt();
                count += 1;
                x += dx / (square * square) + 1.0;
                y += dy / (square * square) + 1.0;
            }
        }
        if count > 0 {
            x = x / count as f64;
            y = y / count as f64;
            let real = Real2D {
                x: 400.0 * x,
                y: 400.0 * y,
            };
            return real;
        } else {
            let real = Real2D {
                x: 400.0 * x,
                y: 400.0 * y,
            };
            return real;
        }
    }

    pub fn cohesion(self, vec: &Vec<Ref<Bird>>) -> Real2D {
        if vec.is_empty() {
            let real = Real2D { x: 0.0, y: 0.0 };
            return real;
        }

        let mut x = 0.0;
        let mut y = 0.0;

        let mut count = 0;

        for i in 0..vec.len() {
            if self != *vec[i] {
                let dx = toroidal_distance(self.pos.x, vec[i].pos.x, WIDTH);
                let dy = toroidal_distance(self.pos.y, vec[i].pos.y, HEIGTH);
                count += 1;
                x += dx;
                y += dy;
            }
        }
        if count > 0 {
            x = x / count as f64;
            y = y / count as f64;
            let real = Real2D {
                x: -x / 10.0,
                y: -y / 10.0,
            };
            return real;
        } else {
            let real = Real2D {
                x: -x / 10.0,
                y: -y / 10.0,
            };
            return real;
        }
    }

    pub fn randomness(self) -> Real2D {
        let mut rng = rand::thread_rng();
        let r1: f64 = rng.gen();
        let x = r1 * 2.0 - 1.0;
        let r2: f64 = rng.gen();
        let y = r2 * 2.0 - 1.0;

        let square = (x * x + y * y).sqrt();
        let real = Real2D {
            x: 0.05 * x / square,
            y: 0.05 * y / square,
        };
        return real;
    }

    pub fn consistency(self, vec: &Vec<Ref<Bird>>) -> Real2D {
        if vec.is_empty() {
            let real = Real2D { x: 0.0, y: 0.0 };
            return real;
        }

        let mut x = 0.0;
        let mut y = 0.0;

        let mut count = 0;

        for i in 0..vec.len() {
            if self != *vec[i] {
                let _dx = toroidal_distance(self.pos.x, vec[i].pos.x, WIDTH);
                let _dy = toroidal_distance(self.pos.y, vec[i].pos.y, HEIGTH);
                count += 1;
                x += self.pos.x;
                y += self.pos.y;
            }
        }
        if count > 0 {
            x = x / count as f64;
            y = y / count as f64;
            let real = Real2D {
                x: -x / count as f64,
                y: y / count as f64,
            };
            return real;
        } else {
            let real = Real2D { x: x, y: y };
            return real;
        }
    }
}

impl Agent for Bird {
    type SimState = BoidsState;

    fn step(&mut self, state:&BoidsState) {
        let vec = state
            .field1
            .get_neighbors_within_distance(self.pos, 10.0);

        let avoid = self.avoidance(&vec);
        let cohe = self.cohesion(&vec);
        let rand = self.randomness();
        let cons = self.consistency(&vec);
        let mom = self.last_d;

        let mut dx = COHESION * cohe.x
            + AVOIDANCE * avoid.x
            + CONSISTENCY * cons.x
            + RANDOMNESS * rand.x
            + MOMENTUM * mom.x;
        let mut dy = COHESION * cohe.y
            + AVOIDANCE * avoid.y
            + CONSISTENCY * cons.y
            + RANDOMNESS * rand.y
            + MOMENTUM * mom.y;

        let dis = (dx * dx + dy * dy).sqrt();
        if dis > 0.0 {
            dx = dx / dis * JUMP;
            dy = dy / dis * JUMP;
        }

        let _lastd = Real2D { x: dx, y: dy };
        let loc_x = toroidal_transform(self.pos.x + dx, WIDTH);
        let loc_y = toroidal_transform(self.pos.y + dy, WIDTH);

        self.pos = Real2D { x: loc_x, y: loc_y };
        drop(vec);
        state
            .field1
            .set_object_location(*self, Real2D { x: loc_x, y: loc_y });
    }
}

impl Hash for Bird {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.id.hash(state);
        //    state.write_u128(self.id);
        //    state.finish();
    }
}

impl Eq for Bird {}

impl PartialEq for Bird {
    fn eq(&self, other: &Bird) -> bool {
        self.id == other.id
    }
}

impl Location2D<Real2D> for Bird {
    fn get_location(self) -> Real2D {
        self.pos
    }

    fn set_location(&mut self, loc: Real2D) {
        self.pos = loc;
    }
}

impl fmt::Display for Bird {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
