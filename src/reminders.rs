//! Erinnerungen: reine Zeitlogik (testbar) und der zentrale Scheduler.
//! Ein einziger Timer wartet auf den nächsten fälligen Zeitpunkt (in Schritten von höchstens 60 s,
//! damit Standby und Zeitumstellung keine Fehlzeiten erzeugen); kein Dauer-Polling ohne Termine.
use crate::{
    model::{self, Remind},
    *,
};
use chrono::{Local, NaiveDateTime, Timelike};

/// Worauf sich eine Erinnerung bezieht.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Update(u64),
    Task(u64, u64),
    Inbox(u64),
}

impl Target {
    pub fn from_date_target(target: DateTarget) -> Option<Self> {
        match target {
            DateTarget::Update(id) => Some(Self::Update(id)),
            DateTarget::Task(uid, tid) => Some(Self::Task(uid, tid)),
            DateTarget::Inbox(id) => Some(Self::Inbox(id)),
            DateTarget::Quick => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Snooze {
    Minutes(i64),
    TomorrowNine,
}

/// Aktion an einem Toast (Erinnerung).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastAction {
    Open(Target),
    Snooze(Target, Snooze),
    Done(Target),
    Today,
}

/// Ruhige Zeiten in Minuten seit Mitternacht; `from > to` läuft über Mitternacht.
#[derive(Clone, Copy)]
pub struct Quiet {
    pub from: u32,
    pub to: u32,
}

fn at_minute(date: NaiveDate, minute: u32) -> NaiveDateTime {
    date.and_hms_opt(minute / 60 % 24, minute % 60, 0)
        .expect("gültige Uhrzeit")
}

/// Liegt `at` in den ruhigen Zeiten, rückt es auf deren Ende.
pub fn shift_out_of_quiet(at: NaiveDateTime, quiet: Option<Quiet>) -> NaiveDateTime {
    let Some(q) = quiet.filter(|q| q.from != q.to) else {
        return at;
    };
    let minute = at.hour() * 60 + at.minute();
    if q.from < q.to {
        if (q.from..q.to).contains(&minute) {
            at_minute(at.date(), q.to)
        } else {
            at
        }
    } else if minute >= q.from {
        at_minute(at.date() + Duration::days(1), q.to)
    } else if minute < q.to {
        at_minute(at.date(), q.to)
    } else {
        at
    }
}

/// Nächster Meldezeitpunkt aller noch nicht gemeldeten Erinnerungen (Schlummern zählt vor `remind_at`).
pub fn next_fire(reminds: &[&Remind], quiet: Option<Quiet>) -> Option<NaiveDateTime> {
    reminds
        .iter()
        .filter_map(|r| r.due_at())
        .map(|at| shift_out_of_quiet(at, quiet))
        .min()
}

pub fn morning(date: NaiveDate) -> NaiveDateTime {
    at_minute(date, 9 * 60)
}

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

impl NoteWarden {
    fn quiet(&self) -> Option<Quiet> {
        let s = &self.workspace.settings;
        s.quiet_on.then_some(Quiet {
            from: s.quiet_from,
            to: s.quiet_to,
        })
    }

    /// Alle Erinnerungen offener Einträge (erledigte, archivierte und gelöschte entfallen).
    pub fn all_reminds(&self) -> Vec<(Target, &Remind)> {
        let mut list = Vec::new();
        for u in self
            .workspace
            .updates
            .iter()
            .filter(|u| u.is_live() && u.status != Status::Erledigt)
        {
            list.push((Target::Update(u.id), &u.remind));
            list.extend(
                u.tasks
                    .iter()
                    .filter(|t| !t.done)
                    .map(|t| (Target::Task(u.id, t.id), &t.remind)),
            );
        }
        list.extend(
            self.workspace
                .standalone_tasks
                .iter()
                .filter(|t| !t.done)
                .map(|t| (Target::Inbox(t.id), &t.remind)),
        );
        list
    }

    pub fn remind_of(&self, target: Target) -> Option<&Remind> {
        self.all_reminds()
            .into_iter()
            .find(|(t, _)| *t == target)
            .map(|(_, r)| r)
    }

    fn remind_mut(&mut self, target: Target) -> Option<&mut Remind> {
        match target {
            Target::Update(id) => self.workspace.update_mut(id).map(|u| &mut u.remind),
            Target::Task(uid, tid) => self.task_mut(uid, tid).map(|t| &mut t.remind),
            Target::Inbox(id) => self
                .workspace
                .standalone_tasks
                .iter_mut()
                .find(|t| t.id == id)
                .map(|t| &mut t.remind),
        }
    }

    pub fn target_title(&self, target: Target) -> String {
        let title = match target {
            Target::Update(id) => self.workspace.update(id).map(|u| u.title.clone()),
            Target::Task(uid, tid) => self
                .workspace
                .update(uid)
                .and_then(|u| u.tasks.iter().find(|t| t.id == tid))
                .map(|t| t.title.clone()),
            Target::Inbox(id) => self
                .workspace
                .standalone_tasks
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.title.clone()),
        };
        title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Ohne Titel".into())
    }

    /// Ein Satz zu Datum und Projekt für die Benachrichtigung.
    fn target_context(&self, target: Target) -> String {
        let (project, date, parent) = match target {
            Target::Update(id) => self
                .workspace
                .update(id)
                .map(|u| (u.project_id, u.planned_for, None)),
            Target::Task(uid, tid) => self.workspace.update(uid).map(|u| {
                (
                    u.project_id,
                    u.tasks
                        .iter()
                        .find(|t| t.id == tid)
                        .and_then(|t| t.planned_for)
                        .or(u.planned_for),
                    Some(u.title.clone()),
                )
            }),
            Target::Inbox(id) => self
                .workspace
                .standalone_tasks
                .iter()
                .find(|t| t.id == id)
                .map(|t| (t.project_id, t.date, None)),
        }
        .unwrap_or((None, None, None));
        let mut parts = Vec::new();
        if let Some(parent) = parent.filter(|p| !p.trim().is_empty()) {
            parts.push(format!("Aufgabe in {parent}"));
        }
        if let Some(project) =
            project.and_then(|id| self.workspace.projects.iter().find(|p| p.id == id))
        {
            parts.push(project.name.clone());
        }
        if let Some(date) = date {
            parts.push(format!(
                "fällig {}",
                dates::relative(date, self.now_today(), self.workspace.settings.date_format)
            ));
        }
        if parts.is_empty() {
            "Erinnerung".into()
        } else {
            parts.join(" · ")
        }
    }

    pub fn set_remind(
        &mut self,
        target: Target,
        at: Option<NaiveDateTime>,
        cx: &mut Context<Self>,
    ) {
        if !self.writable {
            return;
        }
        let Some(remind) = self.remind_mut(target) else {
            return;
        };
        remind.set(at);
        self.persist(cx);
        self.schedule_reminders(cx);
        cx.notify();
    }

    pub fn snooze(&mut self, target: Target, kind: Snooze, cx: &mut Context<Self>) {
        let at = match kind {
            Snooze::Minutes(minutes) => now() + Duration::minutes(minutes),
            Snooze::TomorrowNine => morning(self.now_today() + Duration::days(1)),
        };
        if let Some(remind) = self.remind_mut(target) {
            remind.snoozed = Some(at);
            remind.reminded = None;
            self.persist(cx);
            self.schedule_reminders(cx);
            cx.notify();
        }
    }

    /// Eigene Uhrzeit "hh:mm" aus dem Datums-Popover: am Fälligkeitstag, sonst heute (oder morgen, wenn vorbei).
    pub fn apply_custom_remind(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(date_target) = self.date_pop.as_ref().map(|pop| pop.target) else {
            return;
        };
        let Some(target) = Target::from_date_target(date_target) else {
            return;
        };
        let text = self.remind_time_input.read(cx).value().trim().to_owned();
        let time = text
            .split_once(':')
            .map_or((text.as_str(), "0"), |(h, m)| (h, m));
        let (Ok(hour), Ok(minute)) = (time.0.parse::<u32>(), time.1.parse::<u32>()) else {
            self.toast("Uhrzeit als hh:mm eingeben, zum Beispiel 14:30", None, cx);
            return;
        };
        let today = self.now_today();
        let day = self
            .target_date(date_target)
            .filter(|d| *d >= today)
            .unwrap_or(today);
        let Some(mut at) = day.and_hms_opt(hour, minute, 0) else {
            self.toast("Uhrzeit als hh:mm eingeben, zum Beispiel 14:30", None, cx);
            return;
        };
        if at <= now() {
            at += Duration::days(1);
        }
        self.remind_time_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.set_remind(target, Some(at), cx);
    }

    /// Menschenlesbar für Tooltips: "Morgen, 9:00".
    pub fn remind_label(&self, at: NaiveDateTime) -> String {
        let today = self.now_today();
        let day = if at.date() == today {
            "Heute".to_owned()
        } else if at.date() == today + Duration::days(1) {
            "Morgen".to_owned()
        } else {
            dates::absolute(at.date(), today, self.workspace.settings.date_format)
        };
        format!("{day}, {}:{:02}", at.hour(), at.minute())
    }

    // ---------- Scheduler ----------

    /// Plant neu: ein Timer bis zum nächsten Ereignis. Ältere Timer enden über die Generationsnummer.
    pub fn schedule_reminders(&mut self, cx: &mut Context<Self>) {
        self.remind_gen += 1;
        let generation = self.remind_gen;
        cx.spawn(async move |this, cx| {
            loop {
                let wait = this
                    .update(cx, |this, cx| {
                        if this.remind_gen != generation {
                            return None;
                        }
                        this.process_reminders(cx);
                        let next = this.next_event()?;
                        let seconds = (next - now()).num_seconds().clamp(1, 60);
                        Some(StdDuration::from_secs(seconds as u64))
                    })
                    .ok()
                    .flatten();
                let Some(wait) = wait else { break };
                cx.background_executor().timer(wait).await;
            }
        })
        .detach();
    }

    fn next_daily(&self) -> Option<NaiveDateTime> {
        let s = &self.workspace.settings;
        if !s.daily_on {
            return None;
        }
        let today = self.now_today();
        let day = if s.last_daily == Some(today) {
            today + Duration::days(1)
        } else {
            today
        };
        Some(shift_out_of_quiet(
            at_minute(day, s.daily_time),
            self.quiet(),
        ))
    }

    fn next_event(&self) -> Option<NaiveDateTime> {
        if !self.workspace.settings.reminders_on || !self.writable {
            return None;
        }
        let reminds: Vec<&Remind> = self.all_reminds().into_iter().map(|(_, r)| r).collect();
        [next_fire(&reminds, self.quiet()), self.next_daily()]
            .into_iter()
            .flatten()
            .min()
    }

    /// Meldet alles Fällige. Mehrere gleichzeitig fällige (Ruhezeit, Standby, App war aus) werden zu einer Meldung.
    fn process_reminders(&mut self, cx: &mut Context<Self>) {
        if !self.workspace.settings.reminders_on || !self.writable {
            return;
        }
        let current = now();
        let quiet = self.quiet();
        let due: Vec<Target> = self
            .all_reminds()
            .into_iter()
            .filter(|(_, r)| {
                r.due_at()
                    .is_some_and(|at| shift_out_of_quiet(at, quiet) <= current)
            })
            .map(|(t, _)| t)
            .collect();
        let daily = self.next_daily().is_some_and(|at| at <= current);
        if due.is_empty() && !daily {
            return;
        }
        for target in &due {
            if let Some(remind) = self.remind_mut(*target) {
                remind.reminded = Some(current);
            }
        }
        if daily {
            self.workspace.settings.last_daily = Some(self.now_today());
        }
        self.persist(cx);
        crate::log::info(&format!(
            "Erinnerungen gemeldet: {}, Tagesübersicht: {daily}",
            due.len()
        ));
        match due.as_slice() {
            [] => {}
            [one] => {
                let (title, body) = (self.target_title(*one), self.target_context(*one));
                self.notify(&title, &body, Some(*one), cx);
            }
            many => {
                let titles: Vec<String> =
                    many.iter().take(3).map(|t| self.target_title(*t)).collect();
                let more = many.len().saturating_sub(3);
                let mut body = titles.join(", ");
                if more > 0 {
                    body.push_str(&format!(" und {more} weitere"));
                }
                self.notify(
                    &model::plural(many.len(), "Erinnerung", "Erinnerungen"),
                    &body,
                    None,
                    cx,
                );
            }
        }
        if daily && let Some(summary) = self.daily_summary() {
            self.notify("Tagesübersicht", &summary, None, cx);
        }
        cx.notify();
    }

    /// "Heute fällig: 3 · Überfällig: 2"; `None`, wenn nichts zutrifft.
    pub fn daily_summary(&self) -> Option<String> {
        let today = self.now_today();
        let (mut due, mut overdue) = (0, 0);
        let mut count = |date: Option<NaiveDate>| match date {
            Some(d) if d == today => due += 1,
            Some(d) if d < today => overdue += 1,
            _ => {}
        };
        for u in self
            .workspace
            .updates
            .iter()
            .filter(|u| u.is_live() && u.status != Status::Erledigt)
        {
            count(u.planned_for);
            u.tasks
                .iter()
                .filter(|t| !t.done)
                .for_each(|t| count(t.planned_for));
        }
        self.workspace
            .standalone_tasks
            .iter()
            .filter(|t| !t.done)
            .for_each(|t| count(t.date));
        let mut parts = Vec::new();
        if due > 0 {
            parts.push(format!("Heute fällig: {due}"));
        }
        if overdue > 0 {
            parts.push(format!("Überfällig: {overdue}"));
        }
        (!parts.is_empty()).then(|| parts.join(" · "))
    }

    /// Im Vordergrund als App-Toast, sonst als Windows-Benachrichtigung.
    pub fn notify(
        &mut self,
        title: &str,
        body: &str,
        target: Option<Target>,
        cx: &mut Context<Self>,
    ) {
        if self.window_active {
            let mut actions = vec![(
                "Öffnen".to_owned(),
                target.map_or(ToastAction::Today, ToastAction::Open),
            )];
            if let Some(target) = target {
                actions.push((
                    "Schlummern".into(),
                    ToastAction::Snooze(target, Snooze::Minutes(10)),
                ));
            }
            self.toast_with_actions(format!("{title}: {body}"), actions, cx);
        } else if !crate::system::notify(title, body, target) {
            crate::log::error("Benachrichtigung konnte nicht angezeigt werden");
        }
    }

    pub fn run_toast_action(
        &mut self,
        id: u64,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(action) = self
            .toasts
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.actions.get(index))
            .map(|(_, a)| *a)
        else {
            return;
        };
        self.toasts.retain(|t| t.id != id);
        match action {
            ToastAction::Open(target) => self.open_target(target, window, cx),
            ToastAction::Snooze(target, kind) => self.snooze(target, kind, cx),
            ToastAction::Done(target) => self.complete_target(target, cx),
            ToastAction::Today => self.show_today(window, cx),
        }
        cx.notify();
    }

    pub fn open_target(&mut self, target: Target, window: &mut Window, cx: &mut Context<Self>) {
        match target {
            Target::Update(id) | Target::Task(id, _) => {
                self.set_view(View::Updates, cx);
                self.select_update(id, window, cx);
                self.detail_open = true;
            }
            Target::Inbox(_) => self.set_view(View::Inbox, cx),
        }
        cx.notify();
    }

    pub fn show_today(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.set_view(View::Today, cx);
    }

    pub fn complete_target(&mut self, target: Target, cx: &mut Context<Self>) {
        match target {
            Target::Task(uid, tid) => {
                if self
                    .workspace
                    .update(uid)
                    .and_then(|u| u.tasks.iter().find(|t| t.id == tid))
                    .is_some_and(|t| !t.done)
                {
                    self.toggle_task(uid, tid, cx);
                }
            }
            Target::Inbox(id) => self.toggle_inbox(id, cx),
            Target::Update(_) => {}
        }
    }

    /// Ereignisse aus Tray, Hotkey, Benachrichtigungen und zweitem Start.
    pub fn handle_system_event(
        &mut self,
        event: crate::system::Event,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::system::Event;
        match event {
            Event::ShowWindow => crate::system::show_main_window(),
            Event::QuickCapture => {
                if let Some((handle, popup)) = self.popup_win.clone() {
                    let _ = handle.update(cx, |_, window, cx| {
                        window.activate_window();
                        popup.update(cx, |popup, cx| popup.create_compact_note(window, cx));
                    });
                    return;
                }
                crate::system::show_main_window();
                self.set_view(View::Today, cx);
                self.focus_quick_add(window, cx);
            }
            Event::Today => {
                crate::system::show_main_window();
                self.show_today(window, cx);
            }
            Event::Compact => self.open_popup(window, cx),
            Event::Quit => {
                self.quitting = true;
                cx.quit();
            }
            Event::UpdaterChanged => cx.notify(),
            Event::Reminder { target, action } => {
                crate::system::show_main_window();
                match (target, action.as_deref()) {
                    (Some(target), Some("snooze10")) => {
                        self.snooze(target, Snooze::Minutes(10), cx)
                    }
                    (Some(target), Some("snooze60")) => {
                        self.snooze(target, Snooze::Minutes(60), cx)
                    }
                    (Some(target), Some("tomorrow")) => {
                        self.snooze(target, Snooze::TomorrowNine, cx)
                    }
                    (Some(target), Some("done")) => self.complete_target(target, cx),
                    (Some(target), _) => self.open_target(target, window, cx),
                    (None, _) => self.show_today(window, cx),
                }
            }
        }
    }

    /// Erinnerungen von heute (auch bereits gemeldete) für die Startseite, nach Zeit sortiert.
    pub fn reminders_today(&self) -> Vec<(Target, NaiveDateTime)> {
        let today = self.now_today();
        let mut list: Vec<_> = self
            .all_reminds()
            .into_iter()
            .filter_map(|(t, r)| {
                r.snoozed
                    .filter(|_| r.reminded.is_none())
                    .or(r.at)
                    .map(|at| (t, at))
            })
            .filter(|(_, at)| at.date() == today)
            .collect();
        list.sort_by_key(|(_, at)| *at);
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, day)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
    }

    #[::core::prelude::v1::test]
    fn next_fire_respects_snooze_reminded_and_quiet_hours() {
        let plain = Remind {
            at: Some(at(16, 9, 0)),
            ..Default::default()
        };
        let snoozed = Remind {
            at: Some(at(15, 9, 0)),
            snoozed: Some(at(16, 12, 0)),
            ..Default::default()
        };
        let done = Remind {
            at: Some(at(10, 9, 0)),
            reminded: Some(at(10, 9, 1)),
            ..Default::default()
        };
        let night = Remind {
            at: Some(at(16, 23, 30)),
            ..Default::default()
        };
        let quiet = Some(Quiet {
            from: 22 * 60,
            to: 7 * 60,
        });
        assert_eq!(
            next_fire(&[&plain, &snoozed, &done], None),
            Some(at(16, 9, 0))
        );
        assert_eq!(next_fire(&[&snoozed, &done], None), Some(at(16, 12, 0)));
        assert_eq!(next_fire(&[&done], None), None);
        assert_eq!(next_fire(&[&night], None), Some(at(16, 23, 30)));
        assert_eq!(next_fire(&[&night], quiet), Some(at(17, 7, 0)));
        let early = Remind {
            at: Some(at(16, 5, 0)),
            ..Default::default()
        };
        assert_eq!(next_fire(&[&early, &plain], quiet), Some(at(16, 7, 0)));
    }
}
