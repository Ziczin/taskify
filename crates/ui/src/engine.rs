//! Игровые правила поверх хранилища: награды, комбо, ачивки, покупки, отмена.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use taskify_core::group::RandomGroup;
use taskify_core::reward::{award, MilestoneHit, Reward};
use taskify_core::task::{Kind, Locales, Schedule, Task};
use taskify_packs::{CmpOp, Combo, Condition, Pack, PackSet, Repeat, Window};
use taskify_store::backup::Tier;
use taskify_store::event::*;
use taskify_store::{EventKind, GameSettings, Store};

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub task_id: String,
    pub awarded: Reward,
    pub milestones: Vec<MilestoneHit>,
    pub combos: Vec<String>,
    pub achievements: Vec<String>,
    pub levels: Vec<u32>,
    pub over_norm: bool,
}

#[derive(Debug, Clone)]
pub struct LastCompletion {
    pub task_id: String,
    pub ts: chrono::DateTime<chrono::FixedOffset>,
    pub awarded: Reward,
    pub date: NaiveDate,
}

pub struct Engine {
    pub store: Store,
    pub packs: PackSet,
    pub last_completion: Option<LastCompletion>,
    /// Папка, из которой загружены паки. Может быть выбрана в настройках.
    pub packs_dir: PathBuf,
}

impl Engine {
    pub fn open(data_root: impl AsRef<Path>) -> Result<Self> {
        Self::open_livespace(data_root, "default")
    }

    /// Открывает именованный лайвспейс; паки берутся из общего `packs`.
    pub fn open_livespace(data_root: impl AsRef<Path>, slug: &str) -> Result<Self> {
        let data_root = data_root.as_ref().to_path_buf();
        Self::open_at(&data_root.join("livespaces").join(slug), &data_root)
    }

    /// Открывает лайвспейс по конкретной папке (может быть вне данных).
    pub fn open_at(livespace: &Path, data_root: &Path) -> Result<Self> {
        Self::open_at_with_packs(livespace, data_root, &data_root.join("packs"))
    }

    /// Как `open_at`, но с отдельной папкой паков.
    pub fn open_at_with_packs(
        livespace: &Path,
        _data_root: &Path,
        packs_dir: &Path,
    ) -> Result<Self> {
        let packs_dir = packs_dir.to_path_buf();
        let live_dir = livespace.to_path_buf();
        let store = Store::open(&live_dir).with_context(|| "не удалось открыть профиль")?;
        let packs = PackSet::load(&packs_dir);
        let mut engine = Self {
            store,
            packs,
            last_completion: None,
            packs_dir,
        };
        engine.apply_disabled_packs();
        engine.seed_seen_tasks()?;
        Ok(engine)
    }

    pub fn reload_packs(&mut self) {
        self.packs = PackSet::load(&self.packs_dir);
        self.apply_disabled_packs();
    }

    /// Переключает папку паков и сразу перечитывает их.
    pub fn set_packs_dir(&mut self, path: impl Into<PathBuf>) {
        self.packs_dir = path.into();
        self.reload_packs();
    }

    /// Выключен ли пак сейчас. Учитывает `default_enabled`: базовый пак без
    /// события включения выключен, даже если его нет в списке `packs_disabled`.
    pub fn is_pack_disabled(&self, id: &str) -> bool {
        match self.packs.packs.get(id) {
            Some(pack) => !pack.enabled,
            None => self.store.state.packs_disabled.contains(id),
        }
    }

    /// Опыт пака: сумма опыта за все выполнения его заданий и случайных
    /// групп. Группа платит по среднему участнику, чтобы уровень не зависел
    /// от случайного выбора.
    pub fn pack_xp(&self, pack: &Pack) -> i64 {
        let mut xp: i64 = pack
            .tasks
            .values()
            .map(|task| {
                let done = self
                    .store
                    .state
                    .tasks
                    .get(&task.id)
                    .map(|state| state.completions_total)
                    .unwrap_or(0);
                (done as i64).saturating_mul(task.reward.xp)
            })
            .sum();
        for group in pack.groups.values() {
            let done = self
                .store
                .state
                .tasks
                .get(&group.id)
                .map(|state| state.completions_total)
                .unwrap_or(0);
            let rewards: Vec<i64> = pack
                .tasks
                .values()
                .filter(|task| task.group.as_deref() == Some(group.id.as_str()))
                .map(|member| member.reward.xp)
                .collect();
            if !rewards.is_empty() {
                xp = xp.saturating_add(
                    (done as i64)
                        .saturating_mul(rewards.iter().sum::<i64>() / rewards.len() as i64),
                );
            }
        }
        xp
    }

    pub fn pack_level(&self, pack: &Pack) -> u32 {
        taskify_core::level_for(self.pack_xp(pack))
    }

    /// Видно ли задание на текущем уровне его пака: уровневые рамки
    /// скрывают задание до `level_from` и после `level_to`.
    pub fn task_level_visible(&self, task: &Task) -> bool {
        let Some(pack_id) = task.id.split('/').next() else {
            return true;
        };
        let Some(pack) = self.packs.packs.get(pack_id) else {
            return true;
        };
        if let Some(group) = pack.groups.get(&task.id) {
            return group.visible(self.pack_level(pack));
        }
        let Some(gate) = pack.task_levels.get(&task.id) else {
            return true;
        };
        gate.visible(self.pack_level(pack))
    }

    pub fn set_pack_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        if enabled {
            let version = self
                .packs
                .packs
                .get(id)
                .map(|pack| pack.version.clone())
                .unwrap_or_default();
            self.record(EventKind::PackInstalled(PackInstalled {
                pack: id.to_string(),
                version,
            }))?;
            self.store.state.packs_disabled.remove(id);
        } else {
            self.record(EventKind::PackDisabled(PackDisabled {
                pack: id.to_string(),
            }))?;
        }
        Ok(())
    }

    pub fn apply_disabled_packs(&mut self) {
        let disabled: Vec<String> = self.store.state.packs_disabled.iter().cloned().collect();
        let installed: Vec<String> = self.store.state.packs.keys().cloned().collect();
        for pack in self.packs.packs.values_mut() {
            let (official, default_enabled) = pack
                .manifest
                .as_ref()
                .map(|manifest| (manifest.official, manifest.default_enabled))
                .unwrap_or((false, false));
            // Базовые паки по умолчанию выключены: включаются только явно
            // (событие об установке/включении в журнале) или если манифест
            // помечен `default_enabled` — так работает обучающий пак.
            let explicitly_enabled = installed.contains(&pack.id);
            pack.enabled = !disabled.contains(&pack.id)
                && (explicitly_enabled || !official || default_enabled);
        }
    }

    /// Помечает задания, с которыми уже взаимодействовали, просмотренными
    /// один раз. Не тронутые задания и новые/открывшиеся позже остаются
    /// «новыми» и подсвечиваются.
    fn seed_seen_tasks(&mut self) -> Result<()> {
        const VERSION: u32 = 2;
        if self.store.state.seen_seed_version >= VERSION {
            return Ok(());
        }
        let mut tasks: Vec<String> = self.store.state.tasks.keys().cloned().collect();
        tasks.sort();
        tasks.dedup();
        self.record(EventKind::TasksSeen(TasksSeen {
            tasks,
            local_date: self.today(),
            seed: true,
            version: VERSION,
        }))?;
        Ok(())
    }

    pub fn is_task_new(&self, task_id: &str) -> bool {
        !self.store.state.viewed_tasks.contains(task_id)
    }

    pub fn mark_task_viewed(&mut self, task_id: &str) -> Result<()> {
        if self.store.state.viewed_tasks.contains(task_id) {
            return Ok(());
        }
        self.record(EventKind::TaskViewed(TaskViewed {
            task_id: task_id.to_string(),
            local_date: self.today(),
        }))?;
        Ok(())
    }

    /// Новые задания, которые сейчас видны в «Сегодня» и о которых ещё не
    /// отправляли системное уведомление.
    pub fn unnotified_new_tasks(&self, date: NaiveDate) -> Vec<String> {
        let mut tasks: Vec<String> = self
            .today_tasks(date)
            .into_iter()
            .filter(|task| self.task_level_visible(task))
            .filter(|task| self.is_due(task, date))
            .filter(|task| self.is_task_new(&task.id))
            .filter(|task| !self.store.state.notified_tasks.contains(&task.id))
            .map(|task| task.id)
            .collect();
        tasks.sort();
        tasks
    }

    pub fn mark_tasks_notified(&mut self, tasks: &[String]) -> Result<()> {
        if tasks.is_empty() {
            return Ok(());
        }
        let date = self.today();
        self.record(EventKind::TasksNotified(TasksNotified {
            tasks: tasks.to_vec(),
            local_date: date,
        }))?;
        Ok(())
    }

    /// Сбрасывает прогресс одного пака. Журнал не переписывается: в него
    /// добавляется событие сброса, поэтому история остаётся полной.
    ///
    /// Ваншоты, которые уже выполнялись, сохраняют результат: сброс не должен
    /// заставлять проходить их заново. Ачивки очищаются только по просьбе.
    pub fn reset_pack(&mut self, id: &str, clear_achievements: bool) -> Result<usize> {
        let pack = self
            .packs
            .packs
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("пак не найден: {id}"))?;
        let mut tasks: Vec<String> = pack.tasks.keys().cloned().collect();
        tasks.extend(pack.groups.keys().cloned());
        let mut keep_tasks: Vec<String> = pack
            .tasks
            .values()
            .filter(|task| {
                task.is_one_shot()
                    && self
                        .store
                        .state
                        .task_ref(&task.id)
                        .map(|state| state.completions_total > 0)
                        .unwrap_or(false)
            })
            .map(|task| task.id.clone())
            .collect();
        keep_tasks.extend(
            pack.groups
                .values()
                .filter(|group| {
                    group.is_one_shot()
                        && self
                            .store
                            .state
                            .task_ref(&group.id)
                            .map(|state| state.completions_total > 0)
                            .unwrap_or(false)
                })
                .map(|group| group.id.clone()),
        );
        let combos: Vec<String> = pack.combos.keys().cloned().collect();
        let achievements = if clear_achievements {
            pack.achievements.keys().cloned().collect()
        } else {
            Vec::new()
        };
        let cleared = tasks
            .iter()
            .filter(|task| {
                !keep_tasks.contains(*task) && self.store.state.tasks.contains_key(*task)
            })
            .count();
        let date = self.today();
        self.record(EventKind::PackReset(PackReset {
            pack: id.to_string(),
            tasks,
            keep_tasks,
            combos,
            achievements,
            local_date: date,
        }))?;
        Ok(cleared)
    }

    pub fn backup_now(&self) -> Result<PathBuf> {
        Ok(self.store.backup_now(Tier::Minutes)?)
    }

    pub fn today(&self) -> NaiveDate {
        self.store
            .rules()
            .days
            .local_date(Local::now().fixed_offset())
    }

    pub fn task(&self, id: &str) -> Option<&Task> {
        self.packs.task(id)
    }

    pub fn group(&self, id: &str) -> Option<&RandomGroup> {
        self.packs.group(id)
    }

    /// Задание-участник случайной группы: само по себе не показывается.
    pub fn is_random_member(&self, id: &str) -> bool {
        self.packs
            .task(id)
            .map(|task| task.group.is_some())
            .unwrap_or(false)
    }

    /// Задание-группа, а не обычное задание.
    pub fn is_group_task(&self, id: &str) -> bool {
        self.packs.group(id).is_some()
    }

    fn group_members(&self, group: &RandomGroup) -> Vec<&Task> {
        self.packs
            .packs
            .values()
            .find(|pack| pack.enabled && pack.groups.contains_key(&group.id))
            .map(|pack| pack.members_of(group))
            .unwrap_or_default()
    }

    /// Может ли участник появиться по своим собственным правилам: цепочка,
    /// окно, срок/ваншот и уровневые рамки. Правила группы сюда не входят.
    fn member_can_appear(&self, member: &Task, date: NaiveDate) -> bool {
        self.task_level_visible(member) && self.is_available(member) && self.is_due(member, date)
    }

    /// Взвешенный выбор из уже подходящих участников. Один и тот же seed
    /// даёт один и тот же выбор; вес по умолчанию — 1.
    fn weighted_member<'a>(&self, members: &[&'a Task], seed: &str) -> Option<&'a Task> {
        let total: f64 = members
            .iter()
            .map(|member| member.group_weight.unwrap_or(1.0))
            .sum();
        if !total.is_finite() || total <= 0.0 {
            return members.first().copied();
        }
        // 53 старших бита хеша дают равномерное число в [0, 1).
        let unit = (stable_hash(seed) >> 11) as f64 / (1u64 << 53) as f64;
        let mut ticket = unit * total;
        for member in members {
            ticket -= member.group_weight.unwrap_or(1.0);
            if ticket < 0.0 {
                return Some(*member);
            }
        }
        members.last().copied()
    }

    /// Выбор нескольких разных участников без повторов. Каждый следующий
    /// выбор получает свой seed, поэтому результат воспроизводим.
    fn weighted_members<'a>(
        &self,
        members: &[&'a Task],
        seed: &str,
        count: usize,
    ) -> Vec<&'a Task> {
        let mut remaining = members.to_vec();
        let mut selected = Vec::new();
        for index in 0..count {
            let Some(member) = self.weighted_member(&remaining, &format!("{seed}:{index}")) else {
                break;
            };
            let Some(position) = remaining
                .iter()
                .position(|candidate| std::ptr::eq(*candidate, member))
            else {
                break;
            };
            remaining.remove(position);
            selected.push(member);
        }
        selected
    }

    /// Карточка группы: выбранный участник плюс условия самой группы.
    pub fn group_task(&self, group: &RandomGroup, date: NaiveDate) -> Option<Task> {
        if group.hidden {
            return None;
        }
        let members = self.group_members(group);
        if members.is_empty() {
            return None;
        }
        let state = self.store.state.task_ref(&group.id);
        let done_today = state.and_then(|state| state.last_done) == Some(date);
        let done = state
            .map(|state| {
                // После выполнения вкладка «Сделано» должна показывать того
                // же участника, а не следующего по счёту.
                if state.last_done == Some(date) {
                    state.completions_total.saturating_sub(1)
                } else {
                    state.completions_total
                }
            })
            .unwrap_or(0);
        // Дневная группа выбирает заново каждый день. Группа с интервалом
        // или ваншотом держит выбор до выполнения, а после него — новый.
        let seed = if group.is_one_shot()
            || matches!(group.schedule, Schedule::Interval { .. } | Schedule::Once)
        {
            format!("{}:{}", group.id, done)
        } else {
            format!("{}:{}", group.id, date)
        };
        // Неподходящие по своим правилам участники не участвуют в розыгрыше:
        // их вес не тратится. Уже выполненная группа остаётся в «Сделано»,
        // даже если правила участника к вечеру изменились.
        let pool: Vec<&Task> = if done_today {
            members.clone()
        } else {
            members
                .iter()
                .copied()
                .filter(|member| self.member_can_appear(member, date))
                .collect()
        };
        let pick = group.pick.max(1);
        if pool.len() < pick {
            return None;
        }
        let selected = self.weighted_members(&pool, &seed, pick);
        if selected.len() < pick {
            return None;
        }
        let member = selected[0];
        let mut resolved = member.clone();
        resolved.id = group.id.clone();
        resolved.kind = group.kind;
        resolved.schedule = group.schedule.clone();
        resolved.window = group.window;
        resolved.after = group.after.clone();
        resolved.group = None;
        resolved.combos = Vec::new();
        resolved.former_ids = Vec::new();
        if pick == 1 {
            // У разных участников могут совпадать локальные id подпунктов.
            // Префикс участника не даёт отметкам перепутаться между выпадениями.
            for subtask in &mut resolved.subtasks {
                subtask.id = format!("{}::{}", member.id, subtask.id);
            }
        } else {
            resolved.title = if group.title.is_empty() {
                member.title.clone()
            } else {
                group.title.clone()
            };
            resolved.description = self.merged_description(&selected);
            resolved.subtasks = self.merged_subtasks(&selected);
        }
        Some(resolved)
    }

    fn merged_description(&self, selected: &[&Task]) -> Locales {
        let mut locales: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for member in selected {
            locales.extend(member.title.keys().cloned());
            locales.extend(member.description.keys().cloned());
        }
        let mut description = Locales::new();
        for locale in locales {
            let mut blocks = Vec::new();
            for member in selected {
                let title = member.title(&locale);
                match member
                    .description
                    .get(&locale)
                    .or_else(|| member.description.get("en"))
                {
                    Some(body) if !body.trim().is_empty() => {
                        blocks.push(format!("{title}\n{body}"));
                    }
                    _ => blocks.push(title),
                }
            }
            description.insert(locale, blocks.join("\n\n"));
        }
        description
    }

    fn merged_subtasks(&self, selected: &[&Task]) -> Vec<taskify_core::task::Subtask> {
        let mut subtasks = Vec::new();
        for member in selected {
            if member.subtasks.is_empty() {
                subtasks.push(taskify_core::task::Subtask {
                    id: member.id.clone(),
                    title: member.title.clone(),
                    description: member.description.clone(),
                    tags: member.tags.clone(),
                    reward: Reward::default(),
                    metrics: member.metrics.clone(),
                });
            } else {
                for subtask in &member.subtasks {
                    let mut subtask = subtask.clone();
                    subtask.id = format!("{}::{}", member.id, subtask.id);
                    subtasks.push(subtask);
                }
            }
        }
        subtasks
    }

    /// Задания, которые видны в «Сегодня»: без участников случайных групп,
    /// с уже выбранным участником вместо карточки группы.
    pub fn today_tasks(&self, date: NaiveDate) -> Vec<Task> {
        let mut tasks: Vec<Task> = self
            .packs
            .tasks()
            .into_iter()
            .filter(|task| !self.is_random_member(&task.id))
            .cloned()
            .collect();
        tasks.extend(
            self.packs
                .groups()
                .into_iter()
                .filter_map(|group| self.group_task(group, date)),
        );
        tasks
    }

    pub fn display_task(&self, id: &str, date: NaiveDate) -> Option<Task> {
        if let Some(task) = self.task(id) {
            return Some(task.clone());
        }
        self.group(id)
            .and_then(|group| self.group_task(group, date))
    }

    fn action_task(&self, id: &str, date: NaiveDate) -> Result<Task> {
        self.display_task(id, date)
            .ok_or_else(|| anyhow!("задание не найдено: {id}"))
    }

    pub fn is_done_today(&self, task_id: &str, date: NaiveDate) -> bool {
        self.store
            .state
            .task_ref(task_id)
            .and_then(|state| state.last_done)
            .map(|last| last == date)
            .unwrap_or(false)
    }

    /// Ваншот уже выполнен когда-либо и больше не должен показываться.
    pub fn is_spent_one_shot(&self, task: &Task) -> bool {
        task.is_one_shot()
            && self
                .store
                .state
                .task_ref(&task.id)
                .and_then(|state| state.last_done)
                .is_some()
    }

    /// Открыта ли задача в цепочке: предыдущая выполнена хотя бы один раз.
    pub fn is_unlocked(&self, task: &Task) -> bool {
        let Some(after) = task.after.as_deref() else {
            return true;
        };
        if after.trim().is_empty() {
            return false;
        }
        let previous = self.packs.task(after).or_else(|| {
            self.packs
                .tasks()
                .into_iter()
                .find(|item| item.id_in_use(after))
        });
        let state_id = if let Some(previous) = previous {
            previous
                .group
                .clone()
                .unwrap_or_else(|| previous.id.clone())
        } else if self.group(after).is_some() {
            after.to_string()
        } else {
            return false;
        };
        self.store
            .state
            .task_ref(&state_id)
            .map(|state| state.completions_total > 0)
            .unwrap_or(false)
    }

    /// Доступно ли задание в указанное время суток.
    pub fn is_available_at(&self, task: &Task, time: NaiveTime) -> bool {
        task.window
            .map(|window| window.contains(time))
            .unwrap_or(true)
    }

    /// Доступно ли задание сейчас по локальным часам.
    pub fn is_available(&self, task: &Task) -> bool {
        self.is_available_at(task, Local::now().time())
    }

    /// Задание взято в работу в этот день.
    pub fn is_started(&self, task_id: &str, date: NaiveDate) -> bool {
        self.store
            .state
            .task_ref(task_id)
            .and_then(|state| state.started)
            .map(|started| started == date)
            .unwrap_or(false)
    }

    pub fn is_rejected_today(&self, task_id: &str, date: NaiveDate) -> bool {
        self.store
            .state
            .task_ref(task_id)
            .and_then(|state| state.last_rejected)
            .map(|last| last == date)
            .unwrap_or(false)
    }

    pub fn start_task(&mut self, task_id: &str) -> Result<()> {
        let task = self.action_task(task_id, self.today())?;
        if !self.is_unlocked(&task) {
            bail!("задание ещё закрыто: сначала выполните предыдущее");
        }
        if !self.is_available(&task) {
            let window = task.window.expect("окно уже проверено");
            bail!("задание доступно с {} до {}", window.from, window.to);
        }
        self.mark_task_viewed(task_id)?;
        let date = self.today();
        self.record(EventKind::TaskStarted(TaskStarted {
            task_id: task_id.to_string(),
            local_date: date,
        }))?;
        Ok(())
    }

    pub fn stop_task(&mut self, task_id: &str) -> Result<()> {
        self.record(EventKind::TaskStopped(TaskStopped {
            task_id: task_id.to_string(),
        }))?;
        Ok(())
    }

    pub fn is_due(&self, task: &Task, date: NaiveDate) -> bool {
        // Закрытая задача цепочки не появляется и не считается оставшейся.
        if !self.is_unlocked(task) {
            return false;
        }
        let Some(state) = self.store.state.task_ref(&task.id) else {
            return true;
        };
        if state.last_done == Some(date) {
            return false;
        }
        // Ваншот не возвращается ни на следующий день, ни позже. Проверяем
        // именно факт выполнения, а не только расписание: тип задания тоже
        // может делать его однократным.
        if task.is_one_shot() && state.last_done.is_some() {
            return false;
        }
        match task.schedule {
            Schedule::Daily => true,
            Schedule::Once => state.last_done.is_none(),
            Schedule::Manual => true,
            Schedule::Interval { days: _ } => {
                let Some(next) = state.next_due else {
                    return true;
                };
                date >= next
            }
        }
    }

    fn record(&mut self, kind: EventKind) -> Result<taskify_store::Event> {
        Ok(self.store.record(kind)?)
    }

    /// Один раз добивает монеты за старые задания, которые выдавали 0.
    ///
    /// Раньше часть паков была почти без монет. Считаем минимум одну монету
    /// за каждое выполненное задание и выдаём разницу отдельным бонусом.
    /// Маркер в профиле не даёт начислить его повторно.
    pub fn credit_legacy_task_coins(&mut self) -> Result<i64> {
        let marker = self
            .store
            .livespace
            .root()
            .join("data")
            .join("legacy-task-coins-v1");
        if marker.exists() {
            return Ok(0);
        }
        let completed = self.store.aggregates.totals.tasks_completed as i64;
        let earned = self.store.aggregates.totals.coins_earned;
        let missing = (completed - earned).max(0);
        if missing > 0 {
            let title = [
                ("ru", "Бонус за прошлые задания"),
                ("en", "Bonus for past tasks"),
            ]
            .into_iter()
            .map(|(locale, text)| (locale.to_string(), text.to_string()))
            .collect();
            self.record(EventKind::ComboAwarded(ComboAwarded {
                combo: "legacy-task-coins".to_string(),
                title,
                awarded: Reward::new(0, missing),
                members: Vec::new(),
                window_key: "once".to_string(),
                local_date: self.today(),
            }))?;
        }
        if let Some(parent) = marker.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(marker, format!("{missing}\n"));
        Ok(missing)
    }

    pub fn complete(&mut self, task_id: &str) -> Result<Outcome> {
        let date = self.today();
        let task = self.action_task(task_id, date)?;
        if !self.is_unlocked(&task) {
            bail!("задание ещё закрыто: сначала выполните предыдущее");
        }
        if !self.is_available(&task) {
            let window = task.window.expect("окно уже проверено");
            bail!("задание доступно с {} до {}", window.from, window.to);
        }
        if self.is_done_today(task_id, date) {
            return Err(anyhow!("уже выполнено сегодня"));
        }
        self.mark_task_viewed(task_id)?;

        let existing = self
            .store
            .state
            .task_ref(task_id)
            .cloned()
            .unwrap_or_default();
        let completions_total = existing.completions_total + 1;
        let streak = if existing.last_done == Some(date - chrono::Duration::days(1)) {
            existing.streak_current + 1
        } else if existing.last_done == Some(date) {
            existing.streak_current
        } else {
            1
        };

        let (awarded, milestones) = award(task.reward, &task.milestones, completions_total);
        let over_norm = self.store.settings.daily_norm > 0
            && self.store.state.day.completed >= self.store.settings.daily_norm;

        let level_before = self.store.state.level;
        let event = self.record(EventKind::TaskCompleted(TaskCompleted {
            task: task.clone(),
            base: task.reward,
            milestones: milestones.clone(),
            awarded,
            completions_total,
            streak,
            local_date: date,
            over_norm,
            combos: task.combos.clone(),
        }))?;

        let combos = self.award_combos(&task, date)?;
        let levels = self.emit_level_ups(level_before, date)?;
        let achievements = self.award_achievements(date)?;

        self.last_completion = Some(LastCompletion {
            task_id: task_id.to_string(),
            ts: event.ts,
            awarded,
            date,
        });

        Ok(Outcome {
            task_id: task_id.to_string(),
            awarded,
            milestones,
            combos,
            achievements,
            levels,
            over_norm,
        })
    }

    pub fn reject(&mut self, task_id: &str, reason: Option<String>) -> Result<()> {
        let date = self.today();
        let task = self.action_task(task_id, date)?;
        self.mark_task_viewed(task_id)?;
        self.record(EventKind::TaskRejected(TaskRejected {
            task,
            local_date: date,
            reason,
        }))?;
        self.last_completion = None;
        Ok(())
    }

    pub fn skip_habit(&mut self, task_id: &str) -> Result<()> {
        let date = self.today();
        self.mark_task_viewed(task_id)?;
        self.record(EventKind::HabitSkipped(HabitSkipped {
            task_id: task_id.to_string(),
            local_date: date,
        }))?;
        Ok(())
    }

    pub fn complete_subtask(&mut self, task_id: &str, subtask_id: &str) -> Result<Reward> {
        let task = self.action_task(task_id, self.today())?;
        let subtask = task
            .subtask(subtask_id)
            .ok_or_else(|| anyhow!("подзадача не найдена: {subtask_id}"))?
            .clone();
        if !self.is_available(&task) {
            let window = task.window.expect("окно уже проверено");
            bail!("задание доступно с {} до {}", window.from, window.to);
        }
        let date = self.today();
        if self
            .store
            .state
            .task_ref(task_id)
            .map(|state| state.subtasks_done.contains(subtask_id))
            .unwrap_or(false)
        {
            bail!("подзадача уже отмечена: {subtask_id}");
        }
        self.mark_task_viewed(task_id)?;
        let awarded = subtask.reward;
        self.record(EventKind::SubtaskCompleted(SubtaskCompleted {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            subtask_id: subtask.id.clone(),
            subtask_title: subtask.title.clone(),
            awarded,
            local_date: date,
        }))?;
        Ok(awarded)
    }

    pub fn undo_last(&mut self) -> Result<bool> {
        let Some(last) = self.last_completion.clone() else {
            return Ok(false);
        };
        let window = self.store.settings.undo_seconds as i64;
        let elapsed = (Local::now().fixed_offset() - last.ts).num_seconds();
        if elapsed > window {
            self.last_completion = None;
            return Ok(false);
        }
        self.record(EventKind::TaskCompletionReverted(CompletionReverted {
            target: last.task_id.clone(),
            awarded: last.awarded,
            local_date: last.date,
        }))?;
        self.last_completion = None;
        Ok(true)
    }

    pub fn undo_seconds_left(&self) -> Option<f32> {
        let last = self.last_completion.as_ref()?;
        let window = self.store.settings.undo_seconds as f32;
        let elapsed = (Local::now().fixed_offset() - last.ts).num_seconds() as f32;
        let left = window - elapsed;
        if left > 0.0 {
            Some(left)
        } else {
            None
        }
    }

    fn window_key(&self, combo: &Combo, date: NaiveDate) -> String {
        match combo.window {
            Window::Day => date.to_string(),
            Window::Week => self.store.rules().days.week_start(date).to_string(),
            Window::Month => format!("{:04}-{:02}", date.year(), date.month()),
        }
    }

    fn member_done_in_window(&self, member: &str, combo: &Combo, date: NaiveDate) -> bool {
        let Some(state) = self.store.state.task_ref(member) else {
            return false;
        };
        let Some(last) = state.last_done else {
            return false;
        };
        match combo.window {
            Window::Day => last == date,
            Window::Week => {
                let start = self.store.rules().days.week_start(date);
                last >= start && last <= date
            }
            Window::Month => last.year() == date.year() && last.month() == date.month(),
        }
    }

    fn award_combos(&mut self, task: &Task, date: NaiveDate) -> Result<Vec<String>> {
        let mut awarded = Vec::new();
        let candidates: Vec<Combo> = task
            .combos
            .iter()
            .filter_map(|id| self.packs.combo(id).cloned())
            .filter(|combo| combo.repeat == Repeat::EachWindow || combo.repeat == Repeat::Once)
            .collect();

        for combo in candidates {
            if !combo
                .members
                .iter()
                .all(|member| self.member_done_in_window(member, &combo, date))
            {
                continue;
            }
            let key = match combo.repeat {
                Repeat::Once => "once".to_string(),
                Repeat::EachWindow => self.window_key(&combo, date),
            };
            let done = self
                .store
                .state
                .combos
                .get(&combo.id)
                .cloned()
                .unwrap_or_default();
            if done.awarded.contains(&key) {
                continue;
            }
            let level_before = self.store.state.level;
            self.record(EventKind::ComboAwarded(ComboAwarded {
                combo: combo.id.clone(),
                title: combo.title.clone(),
                awarded: combo.bonus,
                members: combo.members.clone(),
                window_key: key.clone(),
                local_date: date,
            }))?;
            let entry = self.store.state.combos.entry(combo.id.clone()).or_default();
            entry.awarded.insert(key.clone());
            entry.window = key;
            self.emit_level_ups(level_before, date)?;
            awarded.push(combo.id.clone());
        }
        Ok(awarded)
    }

    pub fn achievement_progress(&self, condition: &Condition) -> (i64, i64) {
        match condition {
            Condition::Leaf { metric, value, .. } => {
                let current = self.store.aggregates.value(metric, &self.store.state);
                (current, *value)
            }
            Condition::All { all_of } => {
                let pairs: Vec<(i64, i64)> = all_of
                    .iter()
                    .map(|c| self.achievement_progress(c))
                    .collect();
                let current = pairs.iter().map(|(c, _)| *c).min().unwrap_or(0);
                let target = pairs.iter().map(|(_, t)| *t).max().unwrap_or(0);
                (current.min(target), target)
            }
            Condition::Any { any_of } => {
                let pairs: Vec<(i64, i64)> = any_of
                    .iter()
                    .map(|c| self.achievement_progress(c))
                    .collect();
                let current = pairs.iter().map(|(c, _)| *c).max().unwrap_or(0);
                let target = pairs.iter().map(|(_, t)| *t).min().unwrap_or(0);
                (current, target)
            }
        }
    }

    fn condition_met(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Leaf { metric, op, value } => {
                let current = self.store.aggregates.value(metric, &self.store.state);
                match op {
                    CmpOp::Gte => current >= *value,
                    CmpOp::Gt => current > *value,
                    CmpOp::Lte => current <= *value,
                    CmpOp::Lt => current < *value,
                    CmpOp::Eq => current == *value,
                }
            }
            Condition::All { all_of } => all_of.iter().all(|c| self.condition_met(c)),
            Condition::Any { any_of } => any_of.iter().any(|c| self.condition_met(c)),
        }
    }

    fn award_achievements(&mut self, date: NaiveDate) -> Result<Vec<String>> {
        let mut unlocked = Vec::new();
        for _ in 0..4 {
            let ready: Vec<(String, Locales, Reward, Option<String>)> = self
                .packs
                .achievements()
                .into_iter()
                .filter(|achievement| !self.store.state.achievements.contains(&achievement.id))
                .filter(|achievement| self.condition_met(&achievement.condition))
                .map(|achievement| {
                    (
                        achievement.id.clone(),
                        achievement.title.clone(),
                        achievement.reward,
                        achievement.sticker.clone(),
                    )
                })
                .collect();
            if ready.is_empty() {
                break;
            }
            for (id, title, reward, sticker) in ready {
                let level_before = self.store.state.level;
                let sticker_id = sticker.clone();
                self.record(EventKind::AchievementUnlocked(AchievementUnlocked {
                    achievement: id.clone(),
                    title,
                    awarded: reward,
                    sticker,
                    local_date: date,
                }))?;
                if let Some(sticker) = sticker_id {
                    self.auto_hang_sticker(&sticker)?;
                }
                self.emit_level_ups(level_before, date)?;
                unlocked.push(id);
            }
        }
        Ok(unlocked)
    }

    fn emit_level_ups(&mut self, level_before: u32, date: NaiveDate) -> Result<Vec<u32>> {
        let mut levels = Vec::new();
        let mut current = level_before;
        while current < self.store.state.level {
            current += 1;
            self.record(EventKind::LevelUp(LevelUp {
                level: current,
                local_date: date,
            }))?;
            levels.push(current);
        }
        Ok(levels)
    }

    pub fn can_buy(&self, unlock: &taskify_packs::Unlock) -> bool {
        match unlock {
            taskify_packs::Unlock::Free => true,
            taskify_packs::Unlock::Coins { price } => self.store.state.coins >= *price,
            taskify_packs::Unlock::Level { level } => self.store.state.level >= *level,
            taskify_packs::Unlock::PackLevel { level } => self.store.state.level >= *level,
            taskify_packs::Unlock::Achievement { id } => self.store.state.achievements.contains(id),
        }
    }

    pub fn purchase(&mut self, token: &str, variant: &str) -> Result<()> {
        let token_def = self
            .packs
            .token(token)
            .ok_or_else(|| anyhow!("токен не найден: {token}"))?;
        let variant_def = token_def
            .variants
            .iter()
            .find(|item| item.id == variant)
            .ok_or_else(|| anyhow!("вариант не найден: {variant}"))?;
        let key = format!("{token}/{variant}");
        if self.store.state.owns(&key) || matches!(variant_def.unlock, taskify_packs::Unlock::Free)
        {
            return Ok(());
        }
        if !self.can_buy(&variant_def.unlock) {
            return Err(anyhow!("вариант ещё недоступен"));
        }
        let price = match variant_def.unlock {
            taskify_packs::Unlock::Coins { price } => price,
            _ => 0,
        };
        let date = self.today();
        self.record(EventKind::Purchase(Purchase {
            token: token.to_string(),
            variant: variant.to_string(),
            title: variant_def.title.clone(),
            price,
            local_date: date,
        }))?;
        Ok(())
    }

    pub fn owned(&self, token: &str, variant: &str) -> bool {
        let key = format!("{token}/{variant}");
        if self.store.state.owns(&key) {
            return true;
        }
        self.packs
            .token_any(token)
            .and_then(|def| def.variants.iter().find(|item| item.id == variant))
            .map(|item| matches!(item.unlock, taskify_packs::Unlock::Free))
            .unwrap_or(false)
    }

    /// Набор куплен целиком.
    pub fn bundle_owned(&self, id: &str) -> bool {
        self.store.state.owns(&format!("bundle/{id}"))
    }

    /// Значение варианта (имя файла картинки, звука или семейство шрифта).
    pub fn variant_value(&self, token: &str, variant: &str) -> Option<String> {
        self.packs
            .token_any(token)?
            .variants
            .iter()
            .find(|item| item.id == variant)
            .map(|item| item.value.clone())
    }

    /// Путь к файлу варианта внутри пака (картинки и звуки).
    pub fn variant_asset(&self, token: &str, variant: &str) -> Option<std::path::PathBuf> {
        let value = self.variant_value(token, variant)?;
        let pack_id = token.split('/').next()?;
        let pack = self.packs.packs.get(pack_id)?;
        Some(pack.root.join(value))
    }

    /// Цена содержимого набора без скидки.
    pub fn bundle_full_price(&self, bundle: &taskify_packs::Bundle) -> i64 {
        let mut total = 0;
        for item in &bundle.tokens {
            let Some((token, variant)) = item.split_once(':') else {
                continue;
            };
            let price = self
                .packs
                .token(token)
                .and_then(|def| def.variants.iter().find(|item| item.id == variant))
                .map(|item| match item.unlock {
                    taskify_packs::Unlock::Coins { price } => price,
                    _ => 0,
                })
                .unwrap_or(0);
            total += price;
        }
        total
    }

    pub fn buy_bundle(&mut self, bundle: &taskify_packs::Bundle) -> Result<()> {
        if self.bundle_owned(&bundle.id) {
            return Ok(());
        }
        if self.store.state.coins < bundle.price {
            anyhow::bail!(
                "не хватает монет: {} < {}",
                self.store.state.coins,
                bundle.price
            );
        }
        let items = bundle
            .tokens
            .iter()
            .filter_map(|item| {
                let (token, variant) = item.split_once(':')?;
                Some(BundleItem {
                    token: token.to_string(),
                    variant: variant.to_string(),
                })
            })
            .collect();
        self.record(EventKind::BundlePurchased(BundlePurchased {
            bundle: bundle.id.clone(),
            price: bundle.price,
            items,
            // Тема — только оформление; стикеры в неё не входят.
            stickers: Vec::new(),
            local_date: self.today(),
        }))?;
        Ok(())
    }

    /// Стикер уже куплен: покупки стикеров лежат в `purchased` как `sticker/<id>`.
    pub fn sticker_owned(&self, sticker: &str) -> bool {
        self.store.state.owns(&format!("sticker/{sticker}"))
    }

    pub fn buy_sticker(&mut self, sticker: &crate::stickers::Sticker) -> Result<()> {
        if self.sticker_owned(&sticker.id) {
            return Ok(());
        }
        if self.store.state.coins < sticker.price {
            bail!("не хватает монет: нужно {}", sticker.price);
        }
        let date = self.today();
        self.record(EventKind::Purchase(Purchase {
            token: "sticker".to_string(),
            variant: sticker.id.clone(),
            title: sticker.name.clone(),
            price: sticker.price,
            local_date: date,
        }))?;
        Ok(())
    }

    /// Вешает награду-стикер на свободную панель, если он ещё нигде не висит.
    fn auto_hang_sticker(&mut self, sticker: &str) -> Result<()> {
        if self.sticker_placed(sticker, "left") || self.sticker_placed(sticker, "right") {
            return Ok(());
        }
        let on = |side: &str| {
            self.store
                .state
                .stickers
                .values()
                .filter(|placement| placement.side == side)
                .count()
        };
        let side = if on("left") <= on("right") {
            "left"
        } else {
            "right"
        };
        self.place_sticker(sticker, side)
    }

    /// Висит ли стикер на указанной боковой панели.
    pub fn sticker_placed(&self, sticker: &str, side: &str) -> bool {
        self.store
            .state
            .stickers
            .contains_key(&taskify_store::state::sticker_slot(side, sticker))
    }

    /// Вешает стикер на боковую панель. Стикер может висеть и слева, и справа.
    pub fn place_sticker(&mut self, sticker: &str, side: &str) -> Result<()> {
        if !self.sticker_owned(sticker) {
            bail!("стикер не куплен");
        }
        if !taskify_store::state::STICKER_SIDES.contains(&side) {
            bail!("неизвестная сторона: {side}");
        }
        if self.sticker_placed(sticker, side) {
            return Ok(());
        }
        let count = self
            .store
            .state
            .stickers
            .values()
            .filter(|placement| placement.side == side)
            .count();
        // Вернувшийся стикер начинает заново: внизу панели, без сдвига,
        // с обычным размером и без поворота.
        let y = (count as f32 + 1.0) / (count as f32 + 2.0);
        let (x, moved, scale, rotation) = (0.5, false, 1.0, 0.0);
        self.record(EventKind::StickerPlaced(StickerPlaced {
            sticker: sticker.to_string(),
            x,
            y,
            side: Some(side.to_string()),
            moved,
            scale,
            rotation,
        }))?;
        Ok(())
    }

    /// Двигает стикер по его панели. Координаты — доли ширины и высоты.
    pub fn move_sticker(&mut self, sticker: &str, side: &str, x: f32, y: f32) -> Result<()> {
        let Some(placement) = self
            .store
            .state
            .stickers
            .get(&taskify_store::state::sticker_slot(side, sticker))
            .cloned()
        else {
            bail!("стикер не висит на этой панели");
        };
        if !taskify_store::state::STICKER_SIDES.contains(&side) {
            bail!("неизвестная сторона: {side}");
        }
        self.record(EventKind::StickerPlaced(StickerPlaced {
            sticker: sticker.to_string(),
            x: x.clamp(0.04, 0.96),
            y: y.clamp(0.04, 0.96),
            side: Some(side.to_string()),
            moved: true,
            scale: placement.scale,
            rotation: placement.rotation,
        }))?;
        Ok(())
    }

    /// Меняет размер и поворот висящего стикера, не трогая его позицию.
    pub fn transform_sticker(
        &mut self,
        sticker: &str,
        side: &str,
        scale: f32,
        rotation: f32,
    ) -> Result<()> {
        let Some(placement) = self
            .store
            .state
            .stickers
            .get(&taskify_store::state::sticker_slot(side, sticker))
            .cloned()
        else {
            bail!("стикер не висит на этой панели");
        };
        self.record(EventKind::StickerPlaced(StickerPlaced {
            sticker: sticker.to_string(),
            x: placement.x,
            y: placement.y,
            side: Some(side.to_string()),
            moved: placement.moved,
            scale: taskify_store::state::clamp_sticker_scale(scale),
            rotation: taskify_store::state::normalize_sticker_rotation(rotation),
        }))?;
        Ok(())
    }

    /// Снимает стикер с панели; вторая панель не затрагивается.
    /// Возврат на панель начинается с чистого состояния, без старого вида.
    pub fn remove_sticker(&mut self, sticker: &str, side: &str) -> Result<()> {
        if !self.sticker_placed(sticker, side) {
            return Ok(());
        }
        self.record(EventKind::StickerRemoved(StickerRemoved {
            sticker: sticker.to_string(),
            side: Some(side.to_string()),
        }))?;
        Ok(())
    }

    pub fn week_goal_reached(&self) -> bool {
        self.store.settings.weekly_xp_goal > 0
            && self.store.state.week.xp >= self.store.settings.weekly_xp_goal
    }

    pub fn close_week_if_needed(&mut self) -> Result<()> {
        if self.store.state.week.closed {
            return Ok(());
        }
        let today = self.today();
        let week_start = self.store.rules().days.week_start(today);
        let Some(state_start) = self.store.state.week.start else {
            return Ok(());
        };
        if state_start >= week_start {
            return Ok(());
        }
        let goal_reached = self.week_goal_reached();
        let bonus = if goal_reached {
            Reward::new(50, 10)
        } else {
            Reward::new(0, 0)
        };
        let level_before = self.store.state.level;
        self.record(EventKind::WeekClosed(WeekClosed {
            week_start: state_start,
            xp: self.store.state.week.xp,
            completed: self.store.state.week.completed,
            goal_reached,
            bonus,
        }))?;
        self.emit_level_ups(level_before, today)?;
        self.store.state.week.closed = true;
        Ok(())
    }

    pub fn settings(&self) -> &GameSettings {
        &self.store.settings
    }

    pub fn set_settings(&mut self, settings: GameSettings) -> Result<()> {
        self.store.save_settings(settings)?;
        Ok(())
    }

    pub fn record_settings_change(&mut self, key: &str, value: &str) -> Result<()> {
        self.record(EventKind::SettingsChanged(SettingsChanged {
            key: key.to_string(),
            value: value.to_string(),
        }))?;
        Ok(())
    }
}

/// Сколько заданий осталось на сегодня.
pub fn remaining_today(engine: &Engine, date: NaiveDate) -> usize {
    engine
        .today_tasks(date)
        .into_iter()
        .filter(|task| engine.task_level_visible(task) && engine.is_due(task, date))
        .count()
}

/// Стабильный FNV-1a: выбор участника не должен зависеть от запуска.
fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

pub fn kind_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Habit => "habit",
        Kind::Repeat => "repeat",
        Kind::Once => "once",
    }
}
