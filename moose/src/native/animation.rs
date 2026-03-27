use serde_json::{json, Value};

use super::actions::DaemonState;

/// Enable the Animation domain and return currently tracked animations.
pub async fn handle_animation_list(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let client = &mgr.client;

    // Enable Animation domain
    client
        .send_command("Animation.enable", Some(json!({})), Some(&session_id))
        .await
        .map_err(|e| format!("failed to enable Animation domain: {e}"))?;

    // Get all animations via Runtime.evaluate (the Animation domain events
    // are async; for a synchronous list, query the document's animations)
    let js = r#"
        (function() {
            var anims = document.getAnimations();
            return anims.map(function(a) {
                return {
                    id: a.id || '',
                    name: (a.animationName || a.transitionProperty || ''),
                    type: a.constructor.name,
                    playState: a.playState,
                    playbackRate: a.playbackRate,
                    currentTime: a.currentTime,
                    startTime: a.startTime,
                    pausedState: a.playState === 'paused',
                    effect: a.effect ? {
                        duration: a.effect.getTiming().duration,
                        delay: a.effect.getTiming().delay,
                        easing: a.effect.getTiming().easing,
                        iterations: a.effect.getTiming().iterations,
                        endDelay: a.effect.getTiming().endDelay
                    } : null,
                    target: a.effect && a.effect.target ? {
                        tagName: a.effect.target.tagName,
                        id: a.effect.target.id || null,
                        className: a.effect.target.className || null
                    } : null
                };
            });
        })()
    "#;

    let result = client
        .send_command(
            "Runtime.evaluate",
            Some(json!({
                "expression": js,
                "returnByValue": true,
            })),
            Some(&session_id),
        )
        .await
        .map_err(|e| format!("failed to get animations: {e}"))?;

    let animations = result
        .get("result")
        .and_then(|r| r.get("value"))
        .cloned()
        .unwrap_or(json!([]));

    let is_json = cmd
        .get("json")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if is_json {
        return Ok(json!({ "animations": animations }));
    }

    // Format text output
    let arr = animations.as_array().unwrap_or(&Vec::new()).clone();
    if arr.is_empty() {
        return Ok(json!({ "text": "No animations running" }));
    }

    let mut lines = Vec::new();
    for anim in &arr {
        let name = anim.get("name").and_then(|v| v.as_str()).unwrap_or("(unnamed)");
        let atype = anim.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let state_str = anim.get("playState").and_then(|v| v.as_str()).unwrap_or("unknown");
        let rate = anim.get("playbackRate").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let current = anim.get("currentTime").and_then(|v| v.as_f64());
        let target = anim.get("target");
        let target_str = if let Some(t) = target {
            let tag = t.get("tagName").and_then(|v| v.as_str()).unwrap_or("");
            let id = t.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if !id.is_empty() {
                format!("{}#{}", tag.to_lowercase(), id)
            } else {
                tag.to_lowercase()
            }
        } else {
            String::new()
        };

        let duration = anim
            .get("effect")
            .and_then(|e| e.get("duration"))
            .and_then(|d| d.as_f64());
        let easing = anim
            .get("effect")
            .and_then(|e| e.get("easing"))
            .and_then(|e| e.as_str())
            .unwrap_or("");

        let mut parts = vec![format!("{name} ({atype})")];
        if !target_str.is_empty() {
            parts.push(format!("on {target_str}"));
        }
        parts.push(format!("state={state_str}"));
        if let Some(dur) = duration {
            parts.push(format!("duration={dur}ms"));
        }
        if let Some(cur) = current {
            parts.push(format!("current={cur:.0}ms"));
        }
        if (rate - 1.0).abs() > 0.01 {
            parts.push(format!("rate={rate}x"));
        }
        if !easing.is_empty() {
            parts.push(format!("easing={easing}"));
        }

        lines.push(parts.join(" "));
    }

    Ok(json!({ "text": lines.join("\n") }))
}

/// Pause all or specific animations.
pub async fn handle_animation_pause(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();

    let id_filter = cmd.get("animationId").and_then(|v| v.as_str()).unwrap_or("");

    let js = if id_filter.is_empty() {
        "document.getAnimations().forEach(function(a) { a.pause(); }); 'ok'".to_string()
    } else {
        format!(
            "var a = document.getAnimations().find(function(a) {{ return a.id === '{}'; }}); if (a) {{ a.pause(); 'ok' }} else {{ 'not found' }}",
            id_filter.replace('\'', "\\'")
        )
    };

    mgr.client
        .send_command(
            "Runtime.evaluate",
            Some(json!({ "expression": js, "returnByValue": true })),
            Some(&session_id),
        )
        .await
        .map_err(|e| format!("failed to pause animations: {e}"))?;

    Ok(json!({ "text": "Done" }))
}

/// Resume all paused animations.
pub async fn handle_animation_resume(_cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();

    mgr.client
        .send_command(
            "Runtime.evaluate",
            Some(json!({
                "expression": "document.getAnimations().forEach(function(a) { a.play(); }); 'ok'",
                "returnByValue": true,
            })),
            Some(&session_id),
        )
        .await
        .map_err(|e| format!("failed to resume animations: {e}"))?;

    Ok(json!({ "text": "Done" }))
}

/// Set playback rate for all animations.
pub async fn handle_animation_slow(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();

    let rate = cmd
        .get("rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1);

    let js = format!(
        "document.getAnimations().forEach(function(a) {{ a.updatePlaybackRate({}); }}); 'ok'",
        rate
    );

    mgr.client
        .send_command(
            "Runtime.evaluate",
            Some(json!({ "expression": js, "returnByValue": true })),
            Some(&session_id),
        )
        .await
        .map_err(|e| format!("failed to set playback rate: {e}"))?;

    Ok(json!({ "text": "Done" }))
}

/// Seek a specific animation to a percentage.
pub async fn handle_animation_seek(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();

    let id = cmd
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("animation id required")?;
    let pct = cmd
        .get("percent")
        .and_then(|v| v.as_f64())
        .unwrap_or(50.0);

    let js = format!(
        r#"(function() {{
            var a = document.getAnimations().find(function(a) {{ return a.id === '{}'; }});
            if (!a) return 'not found';
            var dur = a.effect.getTiming().duration;
            a.currentTime = dur * {} / 100;
            return 'ok';
        }})()"#,
        id.replace('\'', "\\'"),
        pct,
    );

    let result = mgr.client
        .send_command(
            "Runtime.evaluate",
            Some(json!({ "expression": js, "returnByValue": true })),
            Some(&session_id),
        )
        .await
        .map_err(|e| format!("failed to seek animation: {e}"))?;

    let val = result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if val == "not found" {
        return Err(format!("animation '{}' not found", id));
    }

    Ok(json!({ "text": "Done" }))
}

/// Wait for animations on a selector to finish.
pub async fn handle_wait_animation(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();

    let selector = cmd.get("selector").and_then(|v| v.as_str()).unwrap_or("");
    let timeout_ms = cmd
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(25000);

    let js = if selector.is_empty() {
        "Promise.all(document.getAnimations().filter(function(a) { return a.playState === 'running'; }).map(function(a) { return a.finished; })).then(function() { return 'done'; })".to_string()
    } else {
        format!(
            r#"(function() {{
                var el = document.querySelector('{}');
                if (!el) return Promise.resolve('no element');
                var anims = el.getAnimations();
                if (anims.length === 0) return Promise.resolve('no animations');
                return Promise.all(anims.filter(function(a) {{ return a.playState === 'running'; }}).map(function(a) {{ return a.finished; }})).then(function() {{ return 'done'; }});
            }})()"#,
            selector.replace('\'', "\\'")
        )
    };

    let result = mgr.client
        .send_command(
            "Runtime.evaluate",
            Some(json!({
                "expression": js,
                "returnByValue": true,
                "awaitPromise": true,
                "timeout": timeout_ms,
            })),
            Some(&session_id),
        )
        .await
        .map_err(|e| format!("failed to wait for animations: {e}"))?;

    let val = result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(json!({ "text": val }))
}
