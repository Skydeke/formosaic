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

#[derive(Debug, Clone)]
pub struct UiStateMachine {
    screen: UiScreen,
}

impl UiStateMachine {
    pub fn new() -> Self {
        Self { screen: UiScreen::MainMenu }
    }

    pub fn screen(&self) -> UiScreen {
        self.screen
    }

    pub fn handle(&mut self, input: UiInput, solved: bool) -> Vec<UiTransition> {
        match self.screen {
            UiScreen::MainMenu => match input {
                UiInput::PlayLevel(id) => {
                    self.screen = UiScreen::Game;
                    vec![UiTransition::StartLevel(id)]
                }
                UiInput::FetchOnline => vec![UiTransition::FetchOnline],
                UiInput::RandomSaved => vec![UiTransition::RandomSaved],
                _ => Vec::new(),
            },
            UiScreen::Game => match input {
                UiInput::Hint => vec![UiTransition::AdvanceHint],
                UiInput::EscapePressed | UiInput::MenuPressed => {
                    if solved {
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
