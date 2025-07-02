# EVENT-DRIVEN ARCHITECTURE FIX

## Current Broken Architecture

The application currently uses a **continuous polling/rendering loop** that wastes massive CPU cycles:

```rust
// main.rs - BROKEN APPROACH
let tick_rate = Duration::from_millis(250);
loop {
    // ❌ RENDERS EVERY 250ms REGARDLESS OF CHANGES
    terminal.draw(|f| ui::draw(f, &mut app))?;
    
    // ❌ POLLS WITH TIMEOUT - NEVER TRULY IDLE
    if crossterm::event::poll(timeout)? {
        handle_event(event, &mut app, &task_sender);
    }
    
    // ❌ CHECKS BACKGROUND TASKS EVERY LOOP
    while let Ok(result) = result_receiver.try_recv() {
        handle_task_result(&mut app, result);
    }
}
```

**Problems:**
- **80% CPU usage while idle** (200ms work every 250ms)
- **Rebuilds entire UI** every 250ms even when nothing changed
- **Recomputes search results** continuously (190ms each time)
- **No true idle state** - always burning CPU

## Correct Event-Driven Architecture

### 1. Main Loop: True Idle State

```rust
// main.rs - CORRECT APPROACH
enum AppState {
    Idle,                           // No background tasks
    Processing(usize),              // N active background tasks
}

let mut app_state = AppState::Idle;

loop {
    match app_state {
        AppState::Idle => {
            // ✅ BLOCK INDEFINITELY - TRUE IDLE
            let event = crossterm::event::read()?;
            if handle_event(event, &mut app, &task_sender)? {
                // ✅ RENDER ONLY WHEN STATE CHANGED
                terminal.draw(|f| ui::draw(f, &app))?;
            }
        }
        
        AppState::Processing(task_count) => {
            // ✅ POLL ONLY WHEN BACKGROUND TASKS ACTIVE
            if crossterm::event::poll(Duration::from_millis(50))? {
                let event = crossterm::event::read()?;
                if handle_event(event, &mut app, &task_sender)? {
                    terminal.draw(|f| ui::draw(f, &app))?;
                }
            }
            
            // ✅ CHECK BACKGROUND TASKS FREQUENTLY WHEN ACTIVE
            let mut tasks_completed = 0;
            while let Ok(result) = result_receiver.try_recv() {
                handle_task_result(&mut app, result);
                tasks_completed += 1;
                // ✅ RENDER IMMEDIATELY WHEN BACKGROUND TASK COMPLETES
                terminal.draw(|f| ui::draw(f, &app))?;
            }
            
            // Update state based on completed tasks
            if tasks_completed > 0 {
                let new_count = task_count.saturating_sub(tasks_completed);
                app_state = if new_count == 0 {
                    AppState::Idle
                } else {
                    AppState::Processing(new_count)
                };
            }
        }
    }
    
    if app.should_quit {
        break;
    }
}
```

### 2. Event Handling: State Change Detection

```rust
// event.rs - RETURN WHETHER STATE ACTUALLY CHANGED
pub fn handle_event(
    event: crossterm::event::Event, 
    app: &mut App, 
    task_sender: &mpsc::UnboundedSender<Task>
) -> Result<bool> {  // ✅ RETURNS TRUE IF UI NEEDS UPDATE
    
    let state_before = app.get_ui_state_hash();
    
    match event {
        Event::Key(key) => {
            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                    return Ok(false);  // ✅ NO RENDER NEEDED
                }
                KeyCode::Char('/') => {
                    app.start_search_mode();
                    return Ok(true);   // ✅ RENDER NEEDED
                }
                // ... other events
            }
        }
    }
    
    let state_after = app.get_ui_state_hash();
    Ok(state_before != state_after)  // ✅ ONLY RENDER IF CHANGED
}
```

### 3. View Model Caching: Eliminate Redundant Work

```rust
// navigator.rs - CACHED VIEW MODEL
pub struct NavigatorState {
    // ... existing fields
    cached_view_model: Option<NavigatorViewModel>,
    view_model_dirty: bool,
    last_state_hash: u64,
}

impl NavigatorState {
    pub fn build_view_model(&mut self) -> &NavigatorViewModel {
        let current_hash = self.compute_state_hash();
        
        // ✅ ONLY REBUILD IF STATE ACTUALLY CHANGED
        if !self.view_model_dirty || self.last_state_hash == current_hash {
            if let Some(ref cached) = self.cached_view_model {
                log::debug!("View model: using cached (no state change)");
                return cached;
            }
        }
        
        log::debug!("View model: rebuilding due to state change");
        let start = std::time::Instant::now();
        
        // ... expensive computation only when needed
        let view_model = self.rebuild_view_model();
        
        self.cached_view_model = Some(view_model);
        self.view_model_dirty = false;
        self.last_state_hash = current_hash;
        
        log::debug!("View model: rebuilt in {:?}", start.elapsed());
        self.cached_view_model.as_ref().unwrap()
    }
    
    fn compute_state_hash(&self) -> u64 {
        // Fast hash of state that affects view model
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.query.hash(&mut hasher);
        self.selection.hash(&mut hasher);
        self.expanded.len().hash(&mut hasher);  // Just count, not full set
        self.editing_search.hash(&mut hasher);
        hasher.finish()
    }
    
    pub fn invalidate_view_model(&mut self) {
        self.view_model_dirty = true;
    }
}
```

### 4. Background Task State Tracking

```rust
// app.rs - TRACK ACTIVE BACKGROUND TASKS
pub struct App {
    // ... existing fields
    active_background_tasks: usize,
}

impl App {
    pub fn start_background_task(&mut self, task: Task) {
        self.active_background_tasks += 1;
        // Send task...
    }
    
    pub fn complete_background_task(&mut self) {
        self.active_background_tasks = self.active_background_tasks.saturating_sub(1);
    }
    
    pub fn has_active_background_tasks(&self) -> bool {
        self.active_background_tasks > 0
    }
}
```

## Performance Impact

### Before (Broken):
- **CPU Usage Idle**: 80% (200ms work every 250ms)
- **Search Performance**: 190ms repeated every 250ms
- **Memory Usage**: Constantly allocating/deallocating view models
- **Battery Life**: Terrible (continuous CPU work)

### After (Event-Driven):
- **CPU Usage Idle**: ~0% (true idle, no work)
- **Search Performance**: 190ms once, then cached (~1ms)
- **Memory Usage**: Stable (cached view models)
- **Battery Life**: Excellent (no background work)

## Implementation Steps

1. **Phase 1**: Add state change detection to `handle_event()`
2. **Phase 2**: Implement view model caching with dirty flags
3. **Phase 3**: Replace continuous loop with true event-driven loop
4. **Phase 4**: Add background task state tracking
5. **Phase 5**: Remove `tick_rate` completely

## Expected Results

After implementation:
- **Typing 'C' in search**: One 190ms computation, then instant responses
- **Sitting idle**: 0% CPU usage, completely quiet
- **Background tasks**: Responsive UI updates only when tasks complete
- **Battery life**: Dramatically improved
- **UI responsiveness**: Feels instant because no competing background work

## The Fundamental Principle

**TUI apps should be like a light switch - OFF when nothing is happening, ON instantly when something happens.**

The current architecture is like a light that **dims and brightens continuously** even when no one is in the room.