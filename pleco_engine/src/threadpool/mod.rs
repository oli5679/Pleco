//! Contains the ThreadPool and the individual Threads.

pub mod threads;

// TODO: use `parking_lot::RwLock`
use std::sync::{RwLock};
use std::sync::atomic::{AtomicBool,Ordering};
use std::thread::{JoinHandle,self};
use std::sync::mpsc::{channel,Receiver,Sender};
use std::time;

use pleco::tools::pleco_arc::Arc;
use pleco::board::*;
use pleco::core::piece_move::BitMove;


use TT_TABLE;
use root_moves::RootMove;
use root_moves::root_moves_list::RootMoveList;
use root_moves::root_moves_manager::RmManager;
use sync::LockLatch;
use time::uci_timer::*;
use time::time_management::TimeManager;
use search::Searcher;
use tables::pawn_table::PawnTable;
use tables::material::Material;

use self::threads::*;

// Data sent from the main thread to initialize a new search
pub struct ThreadGo {
    limit: Limits,
    board: Board
}

pub enum SendData {
    BestMove(RootMove)
}

/// Global Timer
lazy_static! {
    pub static ref TIMER: TimeManager = TimeManager::uninitialized();
}

pub struct ThreadPool {
    // This is the position information we send to each thread upon
    // starting. Contains stuff like the Board, and the Limit to search
    // to.
    pos_state: Arc<RwLock<Option<ThreadGo>>>,

    // This is all rootmoves for all treads.
    rm_manager: RmManager,

    // Join handle for the main thread.
    main_thread: Option<JoinHandle<()>>,

    // The mainthread will send us information through this! Such as
    // the best move available.
    receiver: Receiver<SendData>,

    // CondVar that the mainthread blocks on. We will notif the main thread
    // to awaken through this.
    main_thread_go: Arc<LockLatch>,

    // Vector of all non-main threads
    threads: Vec<JoinHandle<()>>,

    // Tells all threads to go. This is mostly used by the MainThread, we
    // don't really touch this at all.
    all_thread_go: Arc<LockLatch>,

    // should we print stuff to stdout?
    use_stdout: Arc<AtomicBool>,
}

// Okay, this all looks like madness, but there is some reason to it all.
// Basically, `ThreadPool` manages spawning and despawning threads, as well
// as passing state to / from those threads, telling them to stop, go, drop,
// and lastly determining the "best move" from all the threads.
///
// While we spawn all the other threads, We mostly communicate with the
// MainThread to do anything useful. We let the mainthread handle anything fun.
// The goal of the ThreadPool is to be NON BLOCKING, unless we want to await a
// result.
impl ThreadPool {
    fn init(rx: Receiver<SendData>) -> Self {
        ThreadPool {
            pos_state: Arc::new(RwLock::new(None)),
            rm_manager: RmManager::new(),
            main_thread: None,
            receiver: rx,
            main_thread_go: Arc::new(LockLatch::new()),
            threads: Vec::with_capacity(8),
            all_thread_go: Arc::new(LockLatch::new()),
            use_stdout: Arc::new(AtomicBool::new(false)),
        }
    }

    fn create_thread(&self, id: usize, root_moves: RootMoveList) -> Thread {
        let searcher: Searcher = Searcher {
            limit: Limits::blank(),
            board: Board::default(),
            time_man: &TIMER,
            tt: &TT_TABLE,
            pawns: PawnTable::new(16384),
            material: Material::new(8192),
            id,
            root_moves: root_moves.clone(),
            use_stdout: Arc::clone(&self.use_stdout),
        };
        Thread {
            root_moves: root_moves,
            id: id,
            pos_state: Arc::clone(&self.pos_state),
            cond: Arc::clone(&self.all_thread_go),
            searcher
        }
    }

    fn spawn_main_thread(&mut self, tx: Sender<SendData>) {
        let root_moves = self.rm_manager.add_thread().unwrap();
        let thread = self.create_thread(0, root_moves);
        let main_thread = MainThread {
            per_thread: self.rm_manager.clone(),
            main_thread_go: Arc::clone(&self.main_thread_go),
            sender: tx,
            thread,
            use_stdout: Arc::clone(&self.use_stdout)
        };


        let builder = thread::Builder::new().name(String::from("0"));
        self.main_thread = Some(
            builder.spawn(move || {
                let mut main_thread = main_thread;
                main_thread.main_idle_loop()
            }).unwrap());
    }

    /// Creates a new `ThreadPool`
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let mut pool = ThreadPool::init(rx);
        pool.spawn_main_thread(tx);
        pool
    }

    /// Sets the use of standard out. This can be changed mid search as well.
    pub fn stdout(&mut self, use_stdout: bool) {
        self.use_stdout.store(use_stdout, Ordering::Relaxed)
    }

    /// Sets the thread count of the pool. If num is less than 1, nothing will happen.
    ///
    /// # Safety
    ///
    /// Completely unsafe to use when the pool is searching.
    pub fn set_thread_count(&mut self, num: usize) {
        if num > 0 {
            let curr_size = self.rm_manager.size();
            if num > curr_size {
                self.add_threads(num);
            } else if num < curr_size {
                self.remove_threads(num)
            }
        }
    }

    fn add_threads(&mut self, num: usize) {
        let curr_num: usize = self.rm_manager.size();
        let mut i: usize = curr_num;
        while i < num {
            let root_moves = self.rm_manager.add_thread().unwrap();
            let thread = self.create_thread(i, root_moves);
            let builder = thread::Builder::new().name(i.to_string());
            self.threads.push(builder.spawn(move || {
                let mut current_thread = thread;
                current_thread.idle_loop()
            }).unwrap());
            i += 1;
        }
    }


    fn remove_threads(&mut self, num: usize) {
        let curr_num: usize = self.rm_manager.size();
        let mut i: usize = curr_num;
        while i > num {
            self.rm_manager.remove_thread();
            let thread_handle = self.threads.pop().unwrap();
            thread_handle.join().unwrap();
            i -= 1;
        }
    }


    /// Starts a UCI search. The result will be printed to stdout if the stdout setting
    /// is true.
    pub fn uci_search(&mut self, board: &Board, limits: &PreLimits) {
        {
            let mut thread_go = self.pos_state.write().unwrap();
            *thread_go = Some(ThreadGo {
                board: board.shallow_clone(),
                limit: (limits.clone()).create()
            });
        }
        self.main_thread_go.set();
    }

    /// performs a standard search, and blocks waiting for a returned `BitMove`.
    pub fn search(&mut self, board: &Board, limits: &PreLimits) -> BitMove {
        self.uci_search(&board, &limits);
        self.get_move()
    }

    pub fn get_move(&self) -> BitMove {
        let data = self.receiver.recv().unwrap();
        match data {
            SendData::BestMove(t) => t.bit_move
        }
    }

    pub fn stop_searching(&mut self) {
        self.rm_manager.set_stop(true);
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Store that we are dropping
        self.rm_manager.kill_all();
        thread::sleep(time::Duration::new(0,100));
        self.rm_manager.set_stop(true);

        // Notify the main thread to wakeup and stop
        self.main_thread_go.set();

        // Notify the other threads to wakeup and stop
        self.all_thread_go.set();

        // Join all the handles
        while let Some(thread_handle) = self.threads.pop() {
            thread_handle.join().unwrap();
        }
        self.main_thread.take().unwrap().join().unwrap();
    }
}


