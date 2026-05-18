/// Phi Brain — Local AI intelligence layer for Prime.
///
/// A lightweight local model (Phi-4-mini via Ollama) that provides:
/// - T1: Smart model routing/orchestration
/// - T2+T3: Proofreading + hallucination detection
/// - T4: Smart system guardian
/// - T5+T6: User learning + performance optimization
///
/// All processing is 100% local — no data leaves the machine.

pub mod client;
pub mod orchestrator;
pub mod proofreader;
pub mod guardian;
pub mod learner;
pub mod profile_db;

use std::sync::Arc;
use tokio::sync::RwLock;

use self::client::OllamaClient;
use self::orchestrator::SmartOrchestrator;
use self::proofreader::Proofreader;
use self::guardian::SmartGuardian;
use self::learner::Learner;
use self::profile_db::UserProfile;

/// Core Phi Brain system holding all sub-components.
pub struct PhiBrain {
    pub orchestrator: Arc<SmartOrchestrator>,
    pub proofreader: Arc<Proofreader>,
    pub guardian: Arc<SmartGuardian>,
    pub learner: Arc<Learner>,
    pub profile: Arc<RwLock<UserProfile>>,
    pub client: Arc<OllamaClient>,
}

impl PhiBrain {
    /// Create a new Phi Brain instance.
    ///
    /// Creates a single shared Ollama client used by all subsystems.
    /// Ollama availability is checked lazily on first use.
    pub fn new() -> Self {
        let client = OllamaClient::new().shared();
        let profile = Arc::new(RwLock::new(UserProfile::load_or_default()));

        let orchestrator = Arc::new(SmartOrchestrator::with_client(Arc::clone(&client)));
        let proofreader = Arc::new(Proofreader::with_client(Arc::clone(&client)));
        let guardian = Arc::new(SmartGuardian::with_client(Arc::clone(&client)));
        let learner = Arc::new(Learner::with_client(
            Arc::clone(&client),
            Arc::clone(&profile),
        ));

        Self {
            orchestrator,
            proofreader,
            guardian,
            learner,
            profile,
            client,
        }
    }

    /// Check if Phi Brain is available (Ollama reachable).
    pub async fn is_available(&self) -> bool {
        self.client.check_health().await.is_ok()
    }

    /// Get the current profile maturity (0.0 to 1.0).
    pub async fn profile_maturity(&self) -> f32 {
        self.profile.read().await.profile_maturity
    }
}

/// Validate that a Phi Brain endpoint is local-only (security guard).
pub fn validate_local_endpoint(url: &str) -> bool {
    let is_local = url.contains("localhost")
        || url.contains("127.0.0.1")
        || url.contains("::1")
        || url.contains("0.0.0.0");
    is_local
}
