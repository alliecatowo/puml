/// State frame for `if ... else ... endif` branching.
pub(super) struct IfFrame {
    pub(super) diamond_cx: i32,
    pub(super) diamond_arrow_out: i32,
    pub(super) diamond_next_slot: i32,
    /// The `then (guard)` label from `if (cond) then (guard)`.  Retained so
    /// that when the then-branch is empty (no nodes between the diamond and
    /// the else/endif), the EndIf handler can place the guard on the then-merge
    /// arrow — the only visible then-path in that case.
    pub(super) then_guard: Option<String>,
    pub(super) then_cx: i32,
    pub(super) then_rightmost_cx: i32,
    pub(super) then_end_next_slot: i32,
    pub(super) in_else: bool,
    pub(super) else_cx: i32,
    pub(super) else_start_slot: i32,
}

/// State frame for `fork ... forkagain ... endfork` parallel branching.
pub(super) struct ForkFrame {
    pub(super) fork_node_idx: usize,
    pub(super) fork_cx: i32,
    pub(super) fork_slot_y: i32,
    pub(super) branch_start_y: i32,
    pub(super) is_split: bool,
    pub(super) branches: Vec<ForkBranch>,
    pub(super) current_branch: usize,
    pub(super) fork_again_indices: Vec<usize>,
}

pub(super) struct ForkBranch {
    pub(super) start_node_idx: usize,
    pub(super) end_next_slot: i32,
    pub(super) end_node_idx: Option<usize>,
}

pub(super) fn branch_is_live(branch: &ForkBranch, metas: &[super::super::NodeMeta]) -> bool {
    !branch
        .end_node_idx
        .is_some_and(|idx| super::is_activity_terminal_step(&metas[idx].step_kind))
}

/// State frame for `repeat ... repeatwhile` back-edge tracking.
pub(super) struct RepeatFrame {
    pub(super) body_start_idx: usize,
}

/// Tracks the state needed to wire a `while ... endwhile` back-edge.
///
/// PlantUML semantics:
/// - The `while (cond) is (yes)` diamond is the loop-header.
/// - Body nodes follow in the main flow column.
/// - `endwhile` emits a back-edge from the last body node (arrow_out_y of the
///   node just before EndWhile) back to the diamond's body_start_y.
/// - The exit arrow from the diamond's side (the `is (no)` path) continues
///   straight down, exiting the loop.  We add an extra_arrow for the exit side.
pub(super) struct WhileFrame {
    /// Node index of the WhileStart diamond.
    pub(super) diamond_idx: usize,
    /// cx of the diamond.
    pub(super) diamond_cx: i32,
    /// "is (yes)" guard label to put on the back-loop arrow.
    pub(super) yes_guard: Option<String>,
}
