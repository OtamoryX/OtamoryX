use anyhow::{anyhow, Result};
use oar_ocr::oarocr::OAROCRBuilder;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use crate::models::{OcrModelStatus, OcrSettings, OcrSettingsResponse};

const SETTINGS_KEY: &str = "ocr_settings";

#[derive(Debug, Clone)]
pub struct OcrModelDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub language: &'static str,
    pub version: &'static str,
    pub detector: &'static str,
    pub recognizer: &'static str,
    pub dictionary: &'static str,
}

pub const OCR_MODELS: &[OcrModelDefinition] = &[
    OcrModelDefinition {
        id: "ppocrv5-mobile-zh",
        name: "PP-OCRv5 Mobile 中文",
        language: "zh",
        version: "PP-OCRv5",
        detector: "pp-ocrv5_mobile_det.onnx",
        recognizer: "pp-ocrv5_mobile_rec.onnx",
        dictionary: "ppocrv5_dict.txt",
    },
    OcrModelDefinition {
        id: "ppocrv3-mobile-ja",
        name: "PP-OCRv3 Mobile 日本語",
        language: "ja",
        version: "PP-OCRv3",
        detector: "pp-ocrv5_mobile_det.onnx",
        recognizer: "japan_pp-ocrv3_mobile_rec.onnx",
        dictionary: "ppocr_keys_v1.txt",
    },
    OcrModelDefinition {
        id: "ppocrv5-mobile-en",
        name: "PP-OCRv5 Mobile English",
        language: "en",
        version: "PP-OCRv5",
        detector: "pp-ocrv5_mobile_det.onnx",
        recognizer: "en_pp-ocrv5_mobile_rec.onnx",
        dictionary: "ppocrv5_en_dict.txt",
    },
];

type OcrEngine = Arc<Mutex<oar_ocr::oarocr::OAROCR>>;

struct RuntimeState {
    active_model_id: Option<String>,
    engine: Option<OcrEngine>,
    generation: u64,
    switching: bool,
}

pub struct OcrManager {
    state: Mutex<RuntimeState>,
    operations: Arc<Mutex<HashMap<String, Option<String>>>>,
    settings_lock: tokio::sync::Mutex<()>,
    cache_path: PathBuf,
}

static MANAGER: OnceLock<Arc<OcrManager>> = OnceLock::new();

pub fn init_ocr_manager() -> Arc<OcrManager> {
    MANAGER
        .get_or_init(|| {
            let cache_path = std::env::var_os("OCR_MODEL_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data/models/ocr"));
            if let Err(error) = std::fs::create_dir_all(&cache_path) {
                tracing::warn!(path=%cache_path.display(), %error, "failed to create OCR model directory");
            }
            // oar-ocr resolves its verified model registry through OAR_HOME.
            std::env::set_var("OAR_HOME", &cache_path);
            Arc::new(OcrManager {
                state: Mutex::new(RuntimeState {
                    active_model_id: None,
                    engine: None,
                    generation: 0,
                    switching: false,
                }),
                operations: Arc::new(Mutex::new(HashMap::new())),
                settings_lock: tokio::sync::Mutex::new(()),
                cache_path,
            })
        })
        .clone()
}

pub fn ocr_manager() -> Arc<OcrManager> {
    init_ocr_manager()
}

pub async fn load_ocr_settings(pool: &Pool<Sqlite>) -> Result<OcrSettings> {
    let raw = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(SETTINGS_KEY)
        .fetch_optional(pool)
        .await?;
    let settings = match raw {
        Some(value) => serde_json::from_str(&value)
            .map_err(|error| anyhow!("invalid persisted OCR settings: {error}")),
        None => Ok(OcrSettings::default()),
    }?;
    validate_ocr_settings(&settings)?;
    Ok(settings)
}

async fn save_ocr_settings(pool: &Pool<Sqlite>, settings: &OcrSettings) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(SETTINGS_KEY)
    .bind(serde_json::to_string(settings)?)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn validate_ocr_settings(settings: &OcrSettings) -> Result<()> {
    let image = &settings.image;
    if !(512..=4096).contains(&image.target_long_edge)
        || !(512..=4096).contains(&image.large_image_long_edge)
        || image.large_image_long_edge < image.target_long_edge
    {
        return Err(anyhow!("OCR image long edge must be between 512 and 4096, with the large-image edge no smaller than the target edge"));
    }
    if !(60..=95).contains(&image.jpeg_quality)
        || !(60..=95).contains(&image.large_image_jpeg_quality)
    {
        return Err(anyhow!("OCR JPEG quality must be between 60 and 95"));
    }
    if !(32 * 1024 * 1024..=256 * 1024 * 1024).contains(&image.preferred_decode_bytes)
        || !(64 * 1024 * 1024..=512 * 1024 * 1024).contains(&image.large_image_decode_bytes)
        || image.large_image_decode_bytes < image.preferred_decode_bytes
    {
        return Err(anyhow!(
            "OCR decode budgets are outside the supported safe range"
        ));
    }
    if !(256 * 1024..=16 * 1024 * 1024).contains(&image.max_output_bytes)
        || !(512 * 1024..=32 * 1024 * 1024).contains(&image.large_image_max_output_bytes)
        || image.large_image_max_output_bytes < image.max_output_bytes
    {
        return Err(anyhow!(
            "OCR output budgets are outside the supported safe range"
        ));
    }
    if image.large_image_long_edge == 0 || settings.failure_policy.max_page_retries > 3 {
        return Err(anyhow!(
            "OCR failure policy is outside the supported safe range"
        ));
    }
    Ok(())
}

fn model_definition(model_id: &str) -> Result<&'static OcrModelDefinition> {
    OCR_MODELS
        .iter()
        .find(|model| model.id == model_id)
        .ok_or_else(|| anyhow!("unknown OCR model `{model_id}`"))
}

fn model_downloaded(model: &OcrModelDefinition) -> bool {
    let cache = oar_ocr::download::cache_dir();
    [model.detector, model.recognizer, model.dictionary]
        .iter()
        .all(|file| cache.join(file).is_file())
}

fn build_engine(model: &OcrModelDefinition) -> Result<oar_ocr::oarocr::OAROCR> {
    OAROCRBuilder::new(model.detector, model.recognizer, model.dictionary)
        .image_batch_size(1)
        .region_batch_size(4)
        .build()
        .map_err(|error| anyhow!("failed to load OCR model `{}`: {error}", model.id))
}

fn operation_error(
    operations: &Mutex<HashMap<String, Option<String>>>,
    model_id: &str,
    error: Option<String>,
) {
    if let Ok(mut operations) = operations.lock() {
        operations.insert(model_id.to_string(), error);
    }
}

impl OcrManager {
    pub fn cache_path(&self) -> &PathBuf {
        &self.cache_path
    }

    pub fn response(&self, settings: &OcrSettings) -> OcrSettingsResponse {
        let active_model_id = settings.active_model_id.clone();
        let (loading_model, runtime_error) = self
            .operations
            .lock()
            .ok()
            .and_then(|operations| {
                operations
                    .iter()
                    .find(|(_, error)| error.is_none())
                    .map(|(model_id, _)| (Some(model_id.clone()), None))
            })
            .unwrap_or((None, None));
        let runtime_active = self
            .state
            .lock()
            .ok()
            .and_then(|state| state.active_model_id.clone());

        let models = OCR_MODELS
            .iter()
            .map(|model| OcrModelStatus {
                id: model.id.to_string(),
                name: model.name.to_string(),
                language: model.language.to_string(),
                version: model.version.to_string(),
                downloaded: model_downloaded(model),
                active: active_model_id == model.id,
                loading: loading_model.as_deref() == Some(model.id),
                error: if runtime_active.as_deref() == Some(model.id) {
                    runtime_error.clone()
                } else {
                    self.operations
                        .lock()
                        .ok()
                        .and_then(|operations| operations.get(model.id).cloned().flatten())
                },
            })
            .collect();
        OcrSettingsResponse {
            enabled: settings.enabled,
            active_model_id,
            cache_path: self.cache_path.display().to_string(),
            image: settings.image.clone(),
            failure_policy: settings.failure_policy.clone(),
            models,
        }
    }

    pub async fn update_settings(
        &self,
        pool: &Pool<Sqlite>,
        update: crate::models::OcrSettingsUpdate,
    ) -> Result<()> {
        let _settings_guard = self.settings_lock.lock().await;
        let mut settings = load_ocr_settings(pool).await?;
        settings.enabled = update.enabled;
        settings.image = update.image;
        settings.failure_policy = update.failure_policy;
        validate_ocr_settings(&settings)?;
        save_ocr_settings(pool, &settings).await?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
        state.generation = state.generation.wrapping_add(1);
        // A setting toggle invalidates any in-flight initialization. Its generation guard keeps
        // the stale builder from installing an engine after the toggle completes.
        state.engine = None;
        state.active_model_id = None;
        state.switching = false;
        Ok(())
    }

    pub async fn download_model(&self, model_id: &str) -> Result<()> {
        let model = model_definition(model_id)?.clone();
        {
            let mut operations = self
                .operations
                .lock()
                .map_err(|_| anyhow!("OCR operation lock poisoned"))?;
            if operations.values().any(|error| error.is_none()) {
                return Err(anyhow!("another OCR model operation is already running"));
            }
            operations.insert(model.id.to_string(), None);
        }
        let operations = self.operations.clone();
        let model_id_owned = model.id.to_string();
        let result = tokio::task::spawn_blocking(move || build_engine(&model).map(|_| ()))
            .await
            .map_err(|error| anyhow!("OCR model download task failed: {error}"))?;
        match &result {
            Ok(()) => operation_error(&operations, &model_id_owned, Some("".to_string())),
            Err(error) => operation_error(&operations, &model_id_owned, Some(error.to_string())),
        }
        if let Ok(mut operations) = self.operations.lock() {
            if operations
                .get(&model_id_owned)
                .is_some_and(|error| error.as_deref() == Some(""))
            {
                operations.remove(&model_id_owned);
            }
        }
        result
    }

    pub async fn activate_model(&self, pool: &Pool<Sqlite>, model_id: &str) -> Result<()> {
        let model = model_definition(model_id)?.clone();
        {
            let mut operations = self
                .operations
                .lock()
                .map_err(|_| anyhow!("OCR operation lock poisoned"))?;
            if operations.values().any(|error| error.is_none()) {
                return Err(anyhow!("another OCR model operation is already running"));
            }
            operations.insert(model.id.to_string(), None);
        }
        let switch_generation = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
            if state.switching {
                if let Ok(mut operations) = self.operations.lock() {
                    operations.remove(model_id);
                }
                return Err(anyhow!("another OCR runtime operation is already running"));
            }
            state.switching = true;
            state.generation
        };
        let result = tokio::task::spawn_blocking(move || build_engine(&model))
            .await
            .map_err(|error| anyhow!("OCR model activation task failed: {error}"))?;
        let engine = match result {
            Ok(engine) => engine,
            Err(error) => {
                operation_error(&self.operations, model_id, Some(error.to_string()));
                if let Ok(mut state) = self.state.lock() {
                    if state.generation == switch_generation {
                        state.switching = false;
                    }
                }
                return Err(error);
            }
        };
        let _settings_guard = self.settings_lock.lock().await;
        let mut settings = match load_ocr_settings(pool).await {
            Ok(settings) => settings,
            Err(error) => {
                operation_error(&self.operations, model_id, Some(error.to_string()));
                if let Ok(mut state) = self.state.lock() {
                    if state.generation == switch_generation {
                        state.switching = false;
                    }
                }
                return Err(error);
            }
        };
        settings.active_model_id = model_id.to_string();
        if let Err(error) = save_ocr_settings(pool, &settings).await {
            operation_error(&self.operations, model_id, Some(error.to_string()));
            if let Ok(mut state) = self.state.lock() {
                if state.generation == switch_generation {
                    state.switching = false;
                }
            }
            return Err(error);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
        if state.generation != switch_generation || !state.switching || !settings.enabled {
            // A concurrent disable/re-enable owns the newer generation. Do not install the
            // engine built by this stale activation; a later enabled request will initialize
            // the selected model through ensure_engine.
            if state.generation == switch_generation && state.switching {
                state.switching = false;
                state.engine = None;
                state.active_model_id = None;
            }
            if let Ok(mut operations) = self.operations.lock() {
                operations.remove(model_id);
            }
            return Ok(());
        }
        state.active_model_id = Some(model_id.to_string());
        state.engine = Some(Arc::new(Mutex::new(engine)));
        state.generation = state.generation.wrapping_add(1);
        state.switching = false;
        if let Ok(mut operations) = self.operations.lock() {
            operations.remove(model_id);
        }
        Ok(())
    }

    async fn ensure_engine(&self, model_id: &str) -> Result<(OcrEngine, u64)> {
        {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
            if state.switching {
                return Err(anyhow!("OCR model is switching; retry the analysis"));
            }
            if state.active_model_id.as_deref() == Some(model_id) {
                if let Some(engine) = &state.engine {
                    return Ok((engine.clone(), state.generation));
                }
            }
        }
        self.activate_runtime_model(model_id).await
    }

    async fn activate_runtime_model(&self, model_id: &str) -> Result<(OcrEngine, u64)> {
        let model = model_definition(model_id)?.clone();
        let runtime_generation = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
            if state.switching {
                return Err(anyhow!("OCR model is switching; retry the analysis"));
            }
            state.switching = true;
            state.generation
        };
        let result = tokio::task::spawn_blocking(move || build_engine(&model))
            .await
            .map_err(|error| anyhow!("OCR model initialization task failed: {error}"))?;
        let engine = match result {
            Ok(engine) => engine,
            Err(error) => {
                if let Ok(mut state) = self.state.lock() {
                    if state.generation == runtime_generation {
                        state.switching = false;
                    }
                }
                return Err(error);
            }
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
        if state.generation != runtime_generation || !state.switching {
            return Err(anyhow!("OCR model changed; retry the analysis"));
        }
        state.active_model_id = Some(model_id.to_string());
        state.engine = Some(Arc::new(Mutex::new(engine)));
        state.generation = state.generation.wrapping_add(1);
        state.switching = false;
        Ok((state.engine.as_ref().unwrap().clone(), state.generation))
    }

    pub async fn recognize_page(
        &self,
        pool: &Pool<Sqlite>,
        data: Vec<u8>,
    ) -> Result<Option<String>> {
        let settings = load_ocr_settings(pool).await?;
        if !settings.enabled {
            return Ok(None);
        }
        let (engine, generation) = self.ensure_engine(&settings.active_model_id).await?;
        let text = tokio::task::spawn_blocking(move || {
            let image = image::load_from_memory(&data)
                .map(|image| image.to_rgb8())
                .map_err(|error| anyhow!("failed to decode OCR image: {error}"))?;
            let engine = engine
                .lock()
                .map_err(|_| anyhow!("OCR engine lock poisoned"))?;
            let result = engine
                .predict(vec![image])
                .map_err(|error| anyhow!("OCR inference failed: {error}"))?;
            Ok::<_, anyhow::Error>(
                result
                    .first()
                    .map(|page| page.concatenated_text("\n"))
                    .unwrap_or_default(),
            )
        })
        .await
        .map_err(|error| anyhow!("OCR inference task failed: {error}"))??;
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
        if state.generation != generation
            || state.switching
            || state.active_model_id.as_deref() != Some(settings.active_model_id.as_str())
        {
            return Err(anyhow!("OCR model changed; retry the analysis"));
        }
        Ok(Some(text))
    }

    pub async fn prepare_analysis(&self, pool: &Pool<Sqlite>) -> Result<Option<(String, u64)>> {
        let settings = load_ocr_settings(pool).await?;
        if !settings.enabled {
            return Ok(None);
        }
        let (_, generation) = self.ensure_engine(&settings.active_model_id).await?;
        Ok(Some((settings.active_model_id, generation)))
    }

    pub async fn validate_analysis_generation(
        &self,
        pool: &Pool<Sqlite>,
        model_id: &str,
        generation: u64,
    ) -> Result<()> {
        let settings = load_ocr_settings(pool).await?;
        if !settings.enabled {
            return Ok(());
        }
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("OCR runtime lock poisoned"))?;
        if state.generation != generation
            || state.switching
            || state.active_model_id.as_deref() != Some(model_id)
            || settings.active_model_id != model_id
        {
            return Err(anyhow!("OCR model changed; retry the analysis"));
        }
        Ok(())
    }
}
