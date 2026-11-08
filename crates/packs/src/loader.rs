//! Загрузка паков с диска и единый набор реестров.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use taskify_core::task::Task;

use crate::entities::*;
use crate::manifest::{Manifest, PACK_SCHEMA};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub severity: Severity,
    pub code: String,
    pub file: String,
    pub field: Option<String>,
    pub message: String,
}

impl Issue {
    pub fn error(code: &str, file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.to_string(),
            file: file.into(),
            field: None,
            message: message.into(),
        }
    }

    pub fn warning(code: &str, file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.to_string(),
            file: file.into(),
            field: None,
            message: message.into(),
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

/// Внутренний id сущности с префиксом пака.
pub fn qualify(pack: &str, id: &str) -> String {
    if id.contains('/') {
        id.to_string()
    } else {
        format!("{pack}/{id}")
    }
}

fn is_system_metric(id: &str) -> bool {
    id.starts_with("sys.")
}

/// Метрики в условиях ачивок живут в пространстве имён пака, как и в заданиях.
fn qualify_condition(pack_id: &str, condition: &mut Condition) {
    match condition {
        Condition::Leaf { metric, .. } => {
            if !is_system_metric(metric) {
                *metric = qualify(pack_id, metric);
            }
        }
        Condition::All { all_of } => {
            for item in all_of {
                qualify_condition(pack_id, item);
            }
        }
        Condition::Any { any_of } => {
            for item in any_of {
                qualify_condition(pack_id, item);
            }
        }
    }
}

fn collect_toml(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_toml(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            out.push(path);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PackSet {
    pub packs: BTreeMap<String, Pack>,
    pub issues: Vec<Issue>,
    pub root: PathBuf,
}

/// Объявление пользовательской метрики в `definitions/metrics/`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MetricDecl {
    pub id: String,
}

impl PackSet {
    pub fn load(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let mut set = Self {
            packs: BTreeMap::new(),
            issues: Vec::new(),
            root: root.clone(),
        };
        let Ok(entries) = std::fs::read_dir(&root) else {
            return set;
        };
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            let name = dir
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') || name.ends_with(".tmp") {
                continue;
            }
            let pack = set.load_pack(&dir);
            if let Some(previous) = set.packs.get(&pack.id) {
                set.issues.push(Issue::error(
                    "PK003",
                    dir.display().to_string(),
                    format!(
                        "пак с id {} уже загружен из {}",
                        pack.id,
                        previous.root.display()
                    ),
                ));
            }
            set.packs.insert(pack.id.clone(), pack);
        }
        set
    }

    fn load_pack(&mut self, dir: &Path) -> Pack {
        let mut pack = Pack {
            root: dir.to_path_buf(),
            enabled: true,
            ..Pack::default()
        };
        let manifest_path = dir.join("pack.toml");
        match std::fs::read_to_string(&manifest_path) {
            Ok(text) => match toml::from_str::<Manifest>(&text) {
                Ok(manifest) => {
                    if manifest.schema > PACK_SCHEMA {
                        self.issues.push(Issue::error(
                            "PK002",
                            manifest_path.display().to_string(),
                            format!(
                                "схема пака {} новее поддерживаемой {}",
                                manifest.schema, PACK_SCHEMA
                            ),
                        ));
                    }
                    pack.id = manifest.id.clone();
                    pack.version = manifest.version.clone();
                    pack.manifest = Some(manifest);
                }
                Err(error) => {
                    self.issues.push(Issue::error(
                        "PK000",
                        manifest_path.display().to_string(),
                        format!("манифест не разбирается: {error}"),
                    ));
                    pack.id = dir
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                }
            },
            Err(_) => {
                self.issues.push(Issue::error(
                    "PK001",
                    manifest_path.display().to_string(),
                    "нет pack.toml",
                ));
                pack.id = dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
            }
        }

        if pack.id.is_empty() {
            pack.id = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
        }

        self.load_definitions(dir, &mut pack);
        self.load_stickers(dir, &mut pack);
        self.load_groups(dir, &mut pack);
        self.load_tasks(dir, &mut pack);
        self.check_groups(&mut pack);
        self.load_combos(dir, &mut pack);
        self.load_achievements(dir, &mut pack);
        self.load_tokens(dir, &mut pack);
        self.load_bundles(dir, &mut pack);
        pack
    }

    fn insert_unique<T>(
        map: &mut BTreeMap<String, T>,
        key: String,
        value: T,
        issues: &mut Vec<Issue>,
        file: &str,
    ) {
        if map.contains_key(&key) {
            issues.push(Issue::error(
                "PK003",
                file,
                format!("повторяющийся id: {key}"),
            ));
        }
        map.insert(key, value);
    }

    fn load_definitions(&mut self, dir: &Path, pack: &mut Pack) {
        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        collect_toml(&dir.join("definitions"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            let folder = file
                .parent()
                .and_then(|parent| parent.file_name())
                .map(|name| name.to_string_lossy().to_string());
            let stem = file
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_default();
            let kind = match folder.as_deref() {
                Some("categories") => "categories".to_string(),
                Some("tags") => "tags".to_string(),
                Some("icons") => "icons".to_string(),
                Some("palette") => "palette".to_string(),
                Some("types") => "types".to_string(),
                Some("metrics") => "metrics".to_string(),
                _ => stem,
            };
            match kind.as_str() {
                "categories" => match toml::from_str::<Category>(&text) {
                    Ok(mut item) => {
                        item.id = qualify(&pack_id, &item.id);
                        Self::insert_unique(
                            &mut pack.categories,
                            item.id.clone(),
                            item,
                            &mut self.issues,
                            &file_name,
                        );
                    }
                    Err(error) => self.parse_issue(&file_name, "категория", &error),
                },
                "tags" => match toml::from_str::<Tag>(&text) {
                    Ok(mut item) => {
                        item.id = qualify(&pack_id, &item.id);
                        Self::insert_unique(
                            &mut pack.tags,
                            item.id.clone(),
                            item,
                            &mut self.issues,
                            &file_name,
                        );
                    }
                    Err(error) => self.parse_issue(&file_name, "тег", &error),
                },
                "icons" => match toml::from_str::<IconDef>(&text) {
                    Ok(mut item) => {
                        item.id = qualify(&pack_id, &item.id);
                        Self::insert_unique(
                            &mut pack.icons,
                            item.id.clone(),
                            item,
                            &mut self.issues,
                            &file_name,
                        );
                    }
                    Err(error) => self.parse_issue(&file_name, "иконка", &error),
                },
                "palette" => match toml::from_str::<PaletteColor>(&text) {
                    Ok(mut item) => {
                        item.id = qualify(&pack_id, &item.id);
                        Self::insert_unique(
                            &mut pack.palette,
                            item.id.clone(),
                            item,
                            &mut self.issues,
                            &file_name,
                        );
                    }
                    Err(error) => self.parse_issue(&file_name, "цвет", &error),
                },
                "types" => match toml::from_str::<TaskType>(&text) {
                    Ok(mut item) => {
                        item.id = qualify(&pack_id, &item.id);
                        Self::insert_unique(
                            &mut pack.types,
                            item.id.clone(),
                            item,
                            &mut self.issues,
                            &file_name,
                        );
                    }
                    Err(error) => self.parse_issue(&file_name, "тип", &error),
                },
                "metrics" => match toml::from_str::<MetricDecl>(&text) {
                    Ok(decl) => pack.metrics_declared.push(qualify(&pack_id, &decl.id)),
                    Err(error) => self.parse_issue(&file_name, "метрика", &error),
                },
                _ => {}
            }
        }
    }

    fn parse_issue(&mut self, file: &str, what: &str, error: &toml::de::Error) {
        self.issues.push(Issue::error(
            "PK000",
            file,
            format!("{what} не разбирается: {error}"),
        ));
    }

    fn load_stickers(&mut self, dir: &Path, pack: &mut Pack) {
        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        // Новый формат: `definitions/stickers`; корневой `stickers` оставлен
        // для паков, которые раскладывают контент по типам папок.
        collect_toml(&dir.join("definitions/stickers"), &mut files);
        collect_toml(&dir.join("stickers"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            match toml::from_str::<StickerDef>(&text) {
                Ok(mut item) => {
                    item.id = qualify(&pack_id, &item.id);
                    Self::insert_unique(
                        &mut pack.stickers,
                        item.id.clone(),
                        item,
                        &mut self.issues,
                        &file_name,
                    );
                }
                Err(error) => self.parse_issue(&file_name, "стикер", &error),
            }
        }
    }

    fn load_groups(&mut self, dir: &Path, pack: &mut Pack) {
        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        collect_toml(&dir.join("groups"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            match toml::from_str::<taskify_core::group::RandomGroup>(&text) {
                Ok(mut group) => {
                    for error in group.validate() {
                        self.issues.push(Issue::error(
                            error.code(),
                            &file_name,
                            format!("{error:?}"),
                        ));
                    }
                    group.id = qualify(&pack_id, &group.id);
                    group.after = group.after.map(|id| {
                        if id.trim().is_empty() {
                            id
                        } else {
                            qualify(&pack_id, &id)
                        }
                    });
                    Self::insert_unique(
                        &mut pack.groups,
                        group.id.clone(),
                        group,
                        &mut self.issues,
                        &file_name,
                    );
                }
                Err(error) => self.parse_issue(&file_name, "случайная группа", &error),
            }
        }
    }

    fn load_tasks(&mut self, dir: &Path, pack: &mut Pack) {
        /// Уровневые рамки задания читаются из того же файла: `level_from`
        /// и `level_to` не входят в `Task`, чтобы не менять его структуру.
        #[derive(Default, serde::Deserialize)]
        struct TaskGateDoc {
            #[serde(default)]
            level_from: Option<u32>,
            #[serde(default)]
            level_to: Option<u32>,
        }

        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        collect_toml(&dir.join("tasks"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            let gate = toml::from_str::<TaskGateDoc>(&text)
                .ok()
                .map(|doc| LevelGate {
                    from: doc.level_from,
                    to: doc.level_to,
                })
                .filter(|gate| !gate.is_empty());
            match toml::from_str::<Task>(&text) {
                Ok(mut task) => {
                    task.id = qualify(&pack_id, &task.id);
                    if let Some(gate) = gate {
                        if gate.from.zip(gate.to).map(|(f, t)| f > t).unwrap_or(false) {
                            self.issues.push(Issue::warning(
                                "W020",
                                &file_name,
                                "уровень появления больше уровня пропадания: задание не появится",
                            ));
                        }
                        pack.task_levels.insert(task.id.clone(), gate);
                    }
                    task.category = task.category.map(|id| qualify(&pack_id, &id));
                    task.tags = task.tags.iter().map(|id| qualify(&pack_id, id)).collect();
                    task.combos = task.combos.iter().map(|id| qualify(&pack_id, id)).collect();
                    task.after = task.after.map(|id| {
                        if id.trim().is_empty() {
                            id
                        } else {
                            qualify(&pack_id, &id)
                        }
                    });
                    if let Some(group) = &mut task.group {
                        *group = qualify(&pack_id, group);
                    }
                    task.icon = task.icon.map(|id| qualify(&pack_id, &id));
                    task.former_ids = task
                        .former_ids
                        .iter()
                        .map(|id| qualify(&pack_id, id))
                        .collect();
                    task.metrics = task
                        .metrics
                        .into_iter()
                        .map(|(key, effect)| {
                            let key = if is_system_metric(&key) {
                                key
                            } else {
                                qualify(&pack_id, &key)
                            };
                            (key, effect)
                        })
                        .collect();
                    for subtask in &mut task.subtasks {
                        subtask.tags = subtask
                            .tags
                            .iter()
                            .map(|id| qualify(&pack_id, id))
                            .collect();
                        subtask.metrics = subtask
                            .metrics
                            .iter()
                            .map(|(key, effect)| {
                                let key = if is_system_metric(key) {
                                    key.clone()
                                } else {
                                    qualify(&pack_id, key)
                                };
                                (key, *effect)
                            })
                            .collect();
                    }
                    Self::insert_unique(
                        &mut pack.tasks,
                        task.id.clone(),
                        task,
                        &mut self.issues,
                        &file_name,
                    );
                }
                Err(error) => self.issues.push(Issue::error(
                    "PK000",
                    &file_name,
                    format!("задание не разбирается: {error}"),
                )),
            }
        }
    }

    /// Проверяет ссылки участников на группы и минимальный состав.
    fn check_groups(&mut self, pack: &mut Pack) {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for task in pack.tasks.values() {
            let Some(group_id) = &task.group else {
                continue;
            };
            if !pack.groups.contains_key(group_id) {
                self.issues.push(Issue::error(
                    "PK017",
                    format!(
                        "tasks/{}.toml",
                        task.id.split('/').nth(1).unwrap_or(&task.id)
                    ),
                    format!("случайная группа не найдена: {group_id}"),
                ));
                continue;
            }
            *counts.entry(group_id.clone()).or_default() += 1;
        }
        for (group_id, group) in &pack.groups {
            if let Some(after) = &group.after {
                if !pack.tasks.contains_key(after) && !pack.groups.contains_key(after) {
                    self.issues.push(Issue::error(
                        "PK017",
                        format!(
                            "groups/{}.toml",
                            group_id.split('/').nth(1).unwrap_or(group_id)
                        ),
                        format!("предыдущее задание группы не найдено: {after}"),
                    ));
                }
            }
            let count = counts.get(group_id).copied().unwrap_or(0);
            if count < 2 {
                self.issues.push(Issue::error(
                    "PK017",
                    format!(
                        "groups/{}.toml",
                        group_id.split('/').nth(1).unwrap_or(group_id)
                    ),
                    format!(
                        "в случайной группе {} меньше двух участников",
                        group.title.values().next().cloned().unwrap_or_default()
                    ),
                ));
            }
        }
        for group_id in pack.groups.keys() {
            if pack.tasks.contains_key(group_id) {
                self.issues.push(Issue::error(
                    "PK018",
                    format!(
                        "groups/{}.toml",
                        group_id.split('/').nth(1).unwrap_or(group_id)
                    ),
                    format!("id группы совпадает с id задания: {group_id}"),
                ));
            }
        }
    }

    fn load_combos(&mut self, dir: &Path, pack: &mut Pack) {
        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        collect_toml(&dir.join("combos"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            match toml::from_str::<Combo>(&text) {
                Ok(mut combo) => {
                    combo.id = qualify(&pack_id, &combo.id);
                    combo.members = combo
                        .members
                        .iter()
                        .map(|id| qualify(&pack_id, id))
                        .collect();
                    Self::insert_unique(
                        &mut pack.combos,
                        combo.id.clone(),
                        combo,
                        &mut self.issues,
                        &file_name,
                    );
                }
                Err(error) => self.issues.push(Issue::error(
                    "PK000",
                    &file_name,
                    format!("комбо не разбирается: {error}"),
                )),
            }
        }
    }

    fn load_achievements(&mut self, dir: &Path, pack: &mut Pack) {
        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        collect_toml(&dir.join("achievements"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            match toml::from_str::<Achievement>(&text) {
                Ok(mut achievement) => {
                    achievement.id = qualify(&pack_id, &achievement.id);
                    qualify_condition(&pack_id, &mut achievement.condition);
                    if let Some(id) = achievement.sticker.take() {
                        let qualified = qualify(&pack_id, &id);
                        achievement.sticker = Some(if pack.stickers.contains_key(&qualified) {
                            qualified
                        } else {
                            id
                        });
                    }
                    Self::insert_unique(
                        &mut pack.achievements,
                        achievement.id.clone(),
                        achievement,
                        &mut self.issues,
                        &file_name,
                    );
                }
                Err(error) => self.issues.push(Issue::error(
                    "PK000",
                    &file_name,
                    format!("ачивка не разбирается: {error}"),
                )),
            }
        }
    }

    fn load_bundles(&mut self, dir: &Path, pack: &mut Pack) {
        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        collect_toml(&dir.join("bundles"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            match toml::from_str::<Bundle>(&text) {
                Ok(mut bundle) => {
                    bundle.id = qualify(&pack_id, &bundle.id);
                    bundle.tokens = bundle
                        .tokens
                        .iter()
                        .map(|item| match item.split_once(':') {
                            Some((token, variant)) => {
                                format!("{}:{}", qualify(&pack_id, token), variant)
                            }
                            None => item.clone(),
                        })
                        .collect();
                    Self::insert_unique(
                        &mut pack.bundles,
                        bundle.id.clone(),
                        bundle,
                        &mut self.issues,
                        &file_name,
                    );
                }
                Err(error) => self.issues.push(Issue::error(
                    "PK000",
                    &file_name,
                    format!("набор не разбирается: {error}"),
                )),
            }
        }
    }

    fn load_tokens(&mut self, dir: &Path, pack: &mut Pack) {
        let pack_id = pack.id.clone();
        let mut files = Vec::new();
        collect_toml(&dir.join("cosmetics"), &mut files);
        for file in files {
            let file_name = file.display().to_string();
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => {
                    self.issues
                        .push(Issue::error("PK000", &file_name, error.to_string()));
                    continue;
                }
            };
            match toml::from_str::<Token>(&text) {
                Ok(mut token) => {
                    token.id = qualify(&pack_id, &token.id);
                    let mut seen = std::collections::BTreeSet::new();
                    for variant in &token.variants {
                        if !seen.insert(variant.id.clone()) {
                            self.issues.push(Issue::error(
                                "PK014",
                                &file_name,
                                format!("повторяющийся вариант {} у {}", variant.id, token.id),
                            ));
                        }
                    }
                    Self::insert_unique(
                        &mut pack.tokens,
                        token.id.clone(),
                        token,
                        &mut self.issues,
                        &file_name,
                    );
                }
                Err(error) => self.issues.push(Issue::error(
                    "PK000",
                    &file_name,
                    format!("косметика не разбирается: {error}"),
                )),
            }
        }
    }

    pub fn task(&self, id: &str) -> Option<&Task> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .find_map(|pack| pack.tasks.get(id))
    }

    pub fn group(&self, id: &str) -> Option<&taskify_core::group::RandomGroup> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .find_map(|pack| pack.groups.get(id))
    }

    pub fn groups(&self) -> Vec<&taskify_core::group::RandomGroup> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.groups.values())
            .collect()
    }

    pub fn tasks(&self) -> Vec<&Task> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.tasks.values())
            .collect()
    }

    pub fn combos(&self) -> Vec<&Combo> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.combos.values())
            .collect()
    }

    pub fn achievements(&self) -> Vec<&Achievement> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.achievements.values())
            .collect()
    }

    pub fn combo(&self, id: &str) -> Option<&Combo> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .find_map(|pack| pack.combos.get(id))
    }

    pub fn achievement(&self, id: &str) -> Option<&Achievement> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .find_map(|pack| pack.achievements.get(id))
    }

    pub fn category(&self, id: &str) -> Option<&Category> {
        self.packs.values().find_map(|pack| pack.categories.get(id))
    }

    pub fn tag(&self, id: &str) -> Option<&Tag> {
        self.packs.values().find_map(|pack| pack.tags.get(id))
    }

    pub fn categories(&self) -> Vec<&Category> {
        let mut items: Vec<&Category> = self
            .packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.categories.values())
            .collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        items
    }

    pub fn tags(&self) -> Vec<&Tag> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.tags.values())
            .collect()
    }

    pub fn tokens(&self) -> Vec<&Token> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.tokens.values())
            .collect()
    }

    pub fn bundles(&self) -> Vec<&Bundle> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.bundles.values())
            .collect()
    }

    pub fn bundle(&self, id: &str) -> Option<&Bundle> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .find_map(|pack| pack.bundles.get(id))
    }

    pub fn token(&self, id: &str) -> Option<&Token> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .find_map(|pack| pack.tokens.get(id))
    }

    /// Стикеры включённых паков.
    pub fn stickers(&self) -> Vec<&StickerDef> {
        self.packs
            .values()
            .filter(|pack| pack.enabled)
            .flat_map(|pack| pack.stickers.values())
            .collect()
    }

    /// Стикеры всех установленных паков, включая выключенные: уже купленный
    /// стикер должен открываться и после выключения пака.
    pub fn stickers_any(&self) -> Vec<&StickerDef> {
        self.packs
            .values()
            .flat_map(|pack| pack.stickers.values())
            .collect()
    }

    pub fn sticker_any(&self, id: &str) -> Option<&StickerDef> {
        self.packs.values().find_map(|pack| pack.stickers.get(id))
    }

    /// Все токены, включая выключенные паки: базовые элементы оформления
    /// должны работать и тогда, когда их пак выключен.
    pub fn tokens_any(&self) -> Vec<&Token> {
        self.packs
            .values()
            .flat_map(|pack| pack.tokens.values())
            .collect()
    }

    /// Токен из любого пака, даже выключенного.
    pub fn token_any(&self, id: &str) -> Option<&Token> {
        self.packs.values().find_map(|pack| pack.tokens.get(id))
    }

    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, text).unwrap();
    }

    fn temp_root() -> PathBuf {
        crate::test_support::temp_root("packs")
    }

    #[test]
    fn random_group_members_are_indexed_and_checked() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/group.toml"),
            r#"
id = "group"
[title]
en = "Random"
"#,
        );
        write(
            &root.join("demo/tasks/a.toml"),
            "id = \"a\"\ngroup = \"group\"\n[title]\nen = \"A\"\n",
        );
        write(
            &root.join("demo/tasks/b.toml"),
            "id = \"b\"\ngroup = \"group\"\n[title]\nen = \"B\"\n",
        );
        let set = PackSet::load(&root);
        let pack = &set.packs["demo"];
        assert!(pack.groups.contains_key("demo/group"));
        assert_eq!(pack.tasks["demo/a"].group.as_deref(), Some("demo/group"));
        assert_eq!(pack.tasks["demo/b"].group.as_deref(), Some("demo/group"));
        assert!(set.issues.iter().all(|issue| issue.code != "PK017"));

        std::fs::remove_file(root.join("demo/tasks/b.toml")).unwrap();
        let set = PackSet::load(&root);
        assert!(set.issues.iter().any(|issue| issue.code == "PK017"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn random_group_id_collision_is_pk018() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/same.toml"),
            "id = \"same\"\n[title]\nen = \"Group\"\n",
        );
        write(
            &root.join("demo/tasks/same.toml"),
            "id = \"same\"\n[title]\nen = \"Task\"\n",
        );
        let set = PackSet::load(&root);
        assert!(
            set.issues.iter().any(|issue| issue.code == "PK018"),
            "{:?}",
            set.issues
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn random_group_missing_prerequisite_is_pk017() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/group.toml"),
            "id = \"group\"\nafter = \"missing\"\n[title]\nen = \"Group\"\n",
        );
        write(
            &root.join("demo/tasks/a.toml"),
            "id = \"a\"\ngroup = \"group\"\n[title]\nen = \"A\"\n",
        );
        write(
            &root.join("demo/tasks/b.toml"),
            "id = \"b\"\ngroup = \"group\"\n[title]\nen = \"B\"\n",
        );
        let set = PackSet::load(&root);
        assert!(
            set.issues
                .iter()
                .any(|issue| issue.code == "PK017" && issue.message.contains("missing")),
            "{:?}",
            set.issues
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_random_groups_are_pk003() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        for file in ["one", "two"] {
            write(
                &root.join(format!("demo/groups/{file}.toml")),
                "id = \"group\"\n[title]\nen = \"Group\"\n",
            );
        }
        let set = PackSet::load(&root);
        assert!(
            set.issues.iter().any(|issue| issue.code == "PK003"),
            "{:?}",
            set.issues
        );
        assert_eq!(set.packs["demo"].groups.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn random_member_weight_is_loaded() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/group.toml"),
            "id = \"group\"\n[title]\nen = \"Group\"\n",
        );
        write(
            &root.join("demo/tasks/a.toml"),
            "id = \"a\"\ngroup = \"group\"\ngroup_weight = 2\n[title]\nen = \"A\"\n",
        );
        write(
            &root.join("demo/tasks/b.toml"),
            "id = \"b\"\ngroup = \"group\"\ngroup_weight = 2.5\n[title]\nen = \"B\"\n",
        );
        let set = PackSet::load(&root);
        let pack = &set.packs["demo"];
        assert_eq!(pack.tasks["demo/a"].group_weight, Some(2.0));
        assert_eq!(pack.tasks["demo/b"].group_weight, Some(2.5));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn random_group_after_alias_is_qualified() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/tasks/before.toml"),
            "id = \"before\"\n[title]\nen = \"Before\"\n",
        );
        write(
            &root.join("demo/groups/group.toml"),
            "id = \"group\"\nprevious = \"before\"\n[title]\nen = \"Group\"\n",
        );
        for id in ["a", "b"] {
            write(
                &root.join(format!("demo/tasks/{id}.toml")),
                &format!("id = \"{id}\"\ngroup = \"group\"\n[title]\nen = \"{id}\"\n"),
            );
        }
        let set = PackSet::load(&root);
        assert_eq!(
            set.packs["demo"].groups["demo/group"].after.as_deref(),
            Some("demo/before")
        );
        assert!(set.issues.iter().all(|issue| issue.code != "PK017"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn random_group_validation_errors_are_reported() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/bad.toml"),
            "id = \"Bad Id\"\nlevel_from = 5\nlevel_to = 2\n",
        );
        let set = PackSet::load(&root);
        for code in ["G001", "G002", "G005"] {
            assert!(
                set.issues.iter().any(|issue| issue.code == code),
                "нет {code}: {:?}",
                set.issues
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn random_group_files_are_optional() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/tasks/t.toml"),
            "id = \"t\"\n[title]\nen = \"T\"\n",
        );
        let set = PackSet::load(&root);
        assert!(set.packs["demo"].groups.is_empty());
        assert!(set.issues.iter().all(|issue| issue.code != "PK017"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn loads_minimal_pack_and_namespaces_ids() {
        let root = temp_root();
        write(
            &root.join("demo/pack.toml"),
            "id = \"demo\"\nversion = \"1.0.0\"\n",
        );
        write(
            &root.join("demo/tasks/t.toml"),
            "id = \"water\"\n[title]\nen = \"Water\"\n",
        );
        let set = PackSet::load(&root);
        assert_eq!(set.packs.len(), 1);
        assert!(set.task("demo/water").is_some());
        assert_eq!(set.tasks().len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn theme_base_is_explicit_or_inferred() {
        let explicit: Bundle = toml::from_str("id = \"t\"\nprice = 1\nbase = \"dark\"\n").unwrap();
        assert_eq!(explicit.theme_base(), ThemeBase::Dark);

        let inferred: Bundle =
            toml::from_str("id = \"t\"\nprice = 1\ntokens = [\"theme:dark\"]\n").unwrap();
        assert_eq!(inferred.theme_base(), ThemeBase::Dark);

        let explicit_wins: Bundle =
            toml::from_str("id = \"t\"\nprice = 1\nbase = \"light\"\ntokens = [\"theme:dark\"]\n")
                .unwrap();
        assert_eq!(explicit_wins.theme_base(), ThemeBase::Light);
    }

    #[test]
    fn gentle_packs_use_chains_and_level_gates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let set = PackSet::load(&root);
        for id in ["greeting", "cleaning", "habits", "inspiration", "social"] {
            assert!(set.packs.contains_key(id), "нет пака {id}");
        }
        let cleaning = set.packs.get("cleaning").unwrap();
        let wash = cleaning.tasks.get("cleaning/kitchen_wash").unwrap();
        assert_eq!(wash.after.as_deref(), Some("cleaning/kitchen_collect"));
        assert!(cleaning.tasks.len() >= 40, "план дома слишком короткий");

        let habits = set.packs.get("habits").unwrap();
        let basic = habits.task_levels.get("habits/morning").unwrap();
        let advanced = habits.task_levels.get("habits/morning_plus").unwrap();
        assert_eq!(basic.to, Some(2));
        assert_eq!(advanced.from, Some(3));
    }

    #[test]
    fn every_builtin_pack_passes_validation() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let set = PackSet::load(&root);
        let issues = crate::validate::validate_set(&set);
        let errors: Vec<&Issue> = issues
            .iter()
            .filter(|issue| issue.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "ошибки встроенных паков: {errors:#?}");
    }

    #[test]
    fn every_builtin_theme_declares_a_base() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let set = PackSet::load(&root);
        let bundles: Vec<&Bundle> = set
            .packs
            .values()
            .flat_map(|pack| pack.bundles.values())
            .collect();
        assert!(!bundles.is_empty(), "во встроенных паках нет тем");
        for bundle in bundles {
            assert!(
                bundle.base.is_some(),
                "у темы {} не указана база",
                bundle.id
            );
        }
    }

    #[test]
    fn missing_manifest_is_reported() {
        let root = temp_root();
        write(&root.join("broken/tasks/t.toml"), "id = \"x\"\n");
        let set = PackSet::load(&root);
        assert!(set.issues.iter().any(|issue| issue.code == "PK001"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_ids_are_reported() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(&root.join("demo/tasks/a.toml"), "id = \"x\"\n");
        write(&root.join("demo/tasks/b.toml"), "id = \"x\"\n");
        let set = PackSet::load(&root);
        assert!(set.issues.iter().any(|issue| issue.code == "PK003"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
