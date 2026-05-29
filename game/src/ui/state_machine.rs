#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiScreen {
    MainMenu,
    Game,
    Credits,
}

#[derive(Debug, Clone)]
pub enum UiInput {
    PlayLevel(String),
    FetchOnline,
    RandomSaved,
    Hint,
    EscapePressed,
    MenuPressed,
    BackToMenuPressed,
    ArtistLinkPressed(String),
}

#[derive(Debug, Clone)]
pub enum UiTransition {
    ShowMainMenu,
    ShowCredits,
    StartLevel(String),
    FetchOnline,
    RandomSaved,
    AdvanceHint,
    OpenArtistLink(String),
}

/// Context passed to the state machine for guard evaluation.
/// This is the single source of truth for UI-guard conditions.
#[derive(Debug, Clone, Default)]
pub struct UiContext {
    /// Whether the current puzzle has been solved.
    pub is_solved: bool,
    /// Whether a level download is in progress.
    pub is_downloading: bool,
    /// Whether a level is being loaded/built.
    pub is_loading: bool,
}

#[derive(Debug, Clone)]
pub struct UiStateMachine {
    screen: UiScreen,
}

impl UiStateMachine {
    pub fn new() -> Self {
        Self {
            screen: UiScreen::MainMenu,
        }
    }

    pub fn screen(&self) -> UiScreen {
        self.screen
    }

    /// Handle a UI input event, applying guard conditions from `ctx`.
    /// Returns the list of transitions to execute on the game layer.
    pub fn handle(&mut self, input: UiInput, ctx: &UiContext) -> Vec<UiTransition> {
        match self.screen {
            UiScreen::MainMenu => match input {
                UiInput::PlayLevel(id) => {
                    self.screen = UiScreen::Game;
                    vec![UiTransition::StartLevel(id)]
                }
                UiInput::FetchOnline => {
                    if !ctx.is_downloading && !ctx.is_loading {
                        self.screen = UiScreen::Game;
                        vec![UiTransition::FetchOnline]
                    } else {
                        Vec::new()
                    }
                }
                UiInput::RandomSaved => {
                    if !ctx.is_downloading && !ctx.is_loading {
                        self.screen = UiScreen::Game;
                        vec![UiTransition::RandomSaved]
                    } else {
                        Vec::new()
                    }
                }
                _ => Vec::new(),
            },
            UiScreen::Game => match input {
                UiInput::Hint => {
                    // No hints once solved (or camera-restoring post-solve).
                    if !ctx.is_loading && !ctx.is_solved {
                        vec![UiTransition::AdvanceHint]
                    } else {
                        Vec::new()
                    }
                }
                UiInput::EscapePressed | UiInput::MenuPressed => {
                    if ctx.is_loading {
                        Vec::new()
                    } else if ctx.is_solved {
                        self.screen = UiScreen::Credits;
                        vec![UiTransition::ShowCredits]
                    } else {
                        self.screen = UiScreen::MainMenu;
                        vec![UiTransition::ShowMainMenu]
                    }
                }
                _ => Vec::new(),
            },
            UiScreen::Credits => match input {
                UiInput::BackToMenuPressed | UiInput::EscapePressed | UiInput::MenuPressed => {
                    self.screen = UiScreen::MainMenu;
                    vec![UiTransition::ShowMainMenu]
                }
                UiInput::ArtistLinkPressed(url) => vec![UiTransition::OpenArtistLink(url)],
                _ => Vec::new(),
            },
        }
    }
}

impl Default for UiStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
