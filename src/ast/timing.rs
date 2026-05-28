#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingDeclKind {
    Concise,
    Robust,
    Clock,
    Binary,
    /// `analog "Label" between MIN and MAX as ID` — continuous-line waveform.
    /// The min/max range is encoded in the `controls` vec as
    /// `__timing:analog_between <min> <max>`.
    Analog,
}

#[derive(Debug, Clone)]
pub struct ActivityStep {
    pub kind: ActivityStepKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityStepKind {
    Start,
    Stop,
    End,
    Action,
    Arrow,
    Connector,
    Note,
    Kill,
    Detach,
    IfStart,
    Else,
    EndIf,
    RepeatStart,
    RepeatWhile,
    WhileStart,
    EndWhile,
    Fork,
    ForkAgain,
    EndFork,
    PartitionStart,
    PartitionEnd,
}
