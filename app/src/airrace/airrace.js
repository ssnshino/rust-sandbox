    function draw() {
      drawGrid();
      drawRunway();
      for (const marker of game.courseMarkers) drawCourseMarker(marker);
      for (const item of game.scenery || []) drawScenery(item);
      for (const gate of game.gates) drawGate(gate, game.gates.indexOf(gate));
      for (const pylon of game.pylons) drawPylon(pylon);
      drawRivals();
      drawPlaneHud();
      drawHud();
      drawDamage();
      drawStartSignal();
    }

    function frame(now) {
      if (!game || !game.running) return;
      resizeCanvas();
      const dt = Math.min(0.05, (now - game.lastMs) / 1000);
      game.lastMs = now;
      update(dt);
      recordGhostSample(now);
      draw();
      updateMeters(now);
      raf = requestAnimationFrame(frame);
    }

    function updateMeters(now) {
      const elapsed = (now - game.totalStartMs) / 1000;
      ui.speed.textContent = `${Math.round(game.speed)} km/h`;
      ui.alt.textContent = `${Math.round(game.altitude)} m`;
      ui.time.textContent = game.round >= GRAND_PRIX_START_ROUND && game.tournament
        ? `R${game.round} ${elapsed.toFixed(2)} / ${game.tournament.playerPoints}pt`
        : `R${game.round} ${elapsed.toFixed(2)}`;
      const rivals = game.rivals || [];
      const gate = game.gates[game.gateIndex];
      const playerDist = gate ? distance2d(gate.x, gate.z, game.planeX, game.planeZ) : 0;
      const playerProgress = game.playerProgress ?? racerProgressValue(game.gateIndex, playerDist);
      const leaderProgress = Math.max(playerProgress, ...rivals.map((rival) => rival.progress || 0));
      const progressGap = game.phase === "landing"
        ? Math.max(0, ...rivals.map((rival) => ((rival.totalTime || 9999) - elapsed) * -100))
        : leaderProgress - playerProgress;
      const guide = guideInfo();
      const fieldSize = currentFieldSize();
      const gateLateral = gate ? Math.abs(lateralError(gate)) : 0;
      const guideActivePhase = game.phase === "race" || game.phase === "landing";
      const offRoute = game.phase === "landing"
        ? Math.abs(landingLateral()) > 110
        : (guide && (guide.arrow === "↻" || guide.arrow === "↺" || guide.dist > 700 || gateLateral > 240));
      if (guideActivePhase && offRoute && guide) {
        ui.guide.style.display = "block";
        const label = game.phase === "landing"
          ? t("runway")
          : t("gate", { index: game.gateIndex + 1 });
        ui.guide.innerHTML = `<span class="guide-arrow">${guide.arrow}</span><span class="guide-meta">${label} / ${Math.round(guide.dist)}m</span>`;
      } else {
        ui.guide.style.display = "none";
      }
      game.hudSnapshot = {
        elapsed,
        fieldSize,
        progressGap,
      };
      if (now > game.messageUntil) {
        ui.message.textContent = "";
      }
      ui.message.style.display = ui.message.textContent ? "block" : "none";
      if (game.phase !== "race" && game.phase !== "landing") {
        game.lowAltitudeWarned = false;
      }
      if (game.altitude >= 58) {
        game.lowAltitudeWarned = false;
      }
      if ((game.phase === "race" || game.phase === "landing")
        && game.altitude < 45
        && !game.lowAltitudeWarned
        && now > game.messageUntil + 220) {
        game.lowAltitudeWarned = true;
        setMessage(t("lowAltitude"), 700);
      }
    }

    function escapeHtml(text) {
      return String(text ?? "")
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;");
    }

    function renderResultWinner(entry) {
      if (!ui.resultWinner || !ui.resultPlanePreview || !ui.resultWinnerName || !ui.resultWinnerAircraft) return;
      if (!entry) {
        ui.resultWinner.style.display = "none";
        ui.resultWinnerName.textContent = "";
        ui.resultWinnerAircraft.textContent = "";
        return;
      }
      ui.resultWinner.style.display = "flex";
      ui.resultWinnerName.textContent = entry.kind === "player" ? t("you") : (entry.name || "---");
      ui.resultWinnerAircraft.textContent = aircraftDisplayName(entry.aircraft);
      const preview = ui.resultPlanePreview;
      const previewCtx = preview.getContext("2d");
      previewCtx.clearRect(0, 0, preview.width, preview.height);
      previewCtx.save();
      previewCtx.translate(preview.width / 2, preview.height / 2 + 4);
      drawBoostEffect(previewCtx, entry.aircraft || AIRCRAFTS[0], 1.5, 0.92);
      drawAircraftShapeTo(
        previewCtx,
        entry.aircraft || AIRCRAFTS[0],
        1.5,
        entry.aircraft?.color || AIRCRAFTS[0].color,
        entry.aircraft?.stroke || AIRCRAFTS[0].stroke
      );
      previewCtx.restore();
    }

    function renderResultRows(target, title, rows, formatter) {
      if (!target) return;
      const visibleRows = (rows || []).slice(0, 10);
      if (visibleRows.length === 0) {
        target.innerHTML = "";
        target.style.display = "none";
        return;
      }
      target.style.display = "grid";
      target.innerHTML = `<div class="result-list-title">${escapeHtml(title)}</div>${visibleRows.map(formatter).join("")}`;
    }

    function renderPodiumAircraft(target, rows) {
      if (!target) return;
      const podium = (rows || []).slice(0, 3);
      if (podium.length === 0) return;
      const header = target.querySelector(".result-list-title");
      const podiumHtml = `<div class="result-podium">${podium.map((row, index) => `
        <div class="result-podium-card ${row.kind === "player" ? "you" : ""}">
          <span class="result-podium-rank">${ordinalLabel(index + 1)}</span>
          <canvas class="result-podium-canvas" width="96" height="36" data-podium-index="${index}"></canvas>
          <span class="result-podium-name">${escapeHtml(row.name)}</span>
          <span class="result-podium-aircraft">${escapeHtml(aircraftDisplayName(row.aircraft) || "")}</span>
        </div>
      `).join("")}</div>`;
      if (header) {
        header.insertAdjacentHTML("afterend", podiumHtml);
      } else {
        target.insertAdjacentHTML("afterbegin", podiumHtml);
      }
      target.querySelectorAll(".result-podium-canvas").forEach((canvasEl) => {
        const idx = Number(canvasEl.dataset.podiumIndex || 0);
        const row = podium[idx];
        const drawCtx = canvasEl.getContext("2d");
        drawCtx.clearRect(0, 0, canvasEl.width, canvasEl.height);
        drawCtx.save();
        drawCtx.translate(canvasEl.width / 2, canvasEl.height / 2 + 2);
        drawBoostEffect(drawCtx, row.aircraft || AIRCRAFTS[0], 0.88, 0.74);
        drawAircraftShapeTo(
          drawCtx,
          row.aircraft || AIRCRAFTS[0],
          0.88,
          row.aircraft?.color || AIRCRAFTS[0].color,
          row.aircraft?.stroke || AIRCRAFTS[0].stroke
        );
        drawCtx.restore();
      });
    }

    function resultRowHtml(row) {
      const aircraftName = row.aircraft ? `<span class="result-line-meta">${escapeHtml(aircraftDisplayName(row.aircraft))}</span>` : "";
      const dnf = row.dnf ? ` <span class="result-line-meta">DNF</span>` : "";
      const points = Number.isFinite(row.points) ? `+${row.points}pt` : "";
      return `<div class="result-line ${row.kind === "player" ? "you" : ""}">
        <span>${row.position}.</span>
        <span class="result-line-name">${escapeHtml(row.name)} ${aircraftName}${dnf}</span>
        <span class="result-line-points">${points}</span>
      </div>`;
    }

    function totalRowHtml(row, index) {
      const aircraftName = row.aircraft ? `<span class="result-line-meta">${escapeHtml(aircraftDisplayName(row.aircraft))}</span>` : "";
      return `<div class="result-line ${row.kind === "player" ? "you" : ""}">
        <span>${index + 1}.</span>
        <span class="result-line-name">${escapeHtml(row.name)} ${aircraftName}</span>
        <span class="result-line-points">${row.points}pt</span>
      </div>`;
    }

    function finishRowHtml(row) {
      const aircraftName = row.aircraft ? `<span class="result-line-meta">${escapeHtml(aircraftDisplayName(row.aircraft))}</span>` : "";
      const timeLabel = row.dnf ? "DNF" : `${formatTime((row.finalTime || 0) * 1000)}`;
      return `<div class="result-line ${row.kind === "player" ? "you" : ""}">
        <span>${row.position}.</span>
        <span class="result-line-name">${escapeHtml(row.name)} ${aircraftName}</span>
        <span class="result-line-points">${timeLabel}</span>
      </div>`;
    }

    function ordinalLabel(rank) {
      if (rank === 1) return "1ST";
      if (rank === 2) return "2ND";
      if (rank === 3) return "3RD";
      return `${rank}TH`;
    }

    function finish(success, reason = "finish") {
      if (!game.running) return;
      game.running = false;
      cancelAnimationFrame(raf);
      stopAudio();
      const elapsedMs = performance.now() - game.totalStartMs;
      game.racePosition = currentRacePosition(elapsedMs / 1000);
      const fieldSize = currentFieldSize();
      const gpSummary = applyTournamentPoints(success || reason === "round_clear", elapsedMs / 1000);
      if (success || reason === "round_clear" || game.ghostSamples.length >= 20) {
        saveGhost({
          name: localStorage.getItem("airrace_name") || "PILOT",
          round: game.round,
          time_ms: Math.round(elapsedMs),
          samples: game.ghostSamples,
          created_at: new Date().toISOString(),
          status: success || reason === "round_clear" ? "finish" : "dnf",
        });
      }
      const recordRound = success ? TOTAL_ROUNDS : Math.max(0, game.round - 1);
      const recordScore = encodeScore(recordRound, elapsedMs);
      const title = ui.result.querySelector("h1");
      ui.guide.style.display = "none";
      ui.nextRound.style.display = "none";
      ui.retry.style.display = "inline-block";
      ui.nameEntry.style.display = "none";
      if (ui.resultStandings) ui.resultStandings.innerHTML = "";
      if (ui.resultTotal) ui.resultTotal.innerHTML = "";
      if (reason === "round_clear") {
        title.textContent = t("roundClear", { round: game.round });
        const result = (game.racePosition || 1) === 1
          ? `<strong>${t("youWin")}</strong>`
          : `<strong>${t("yourPos", { pos: game.racePosition || 1, field: fieldSize })}</strong>`;
        renderResultWinner((gpSummary?.roundRows || [])[0] || raceStandings(true, elapsedMs / 1000)[0]);
        renderResultRows(ui.resultStandings, t("roundResult"), gpSummary?.roundRows || [], resultRowHtml);
        renderResultRows(ui.resultTotal, t("grandPrix"), gpSummary?.tableRows || [], totalRowHtml);
        renderPodiumAircraft(ui.resultTotal, gpSummary?.tableRows || []);
        ui.resultText.innerHTML = gpSummary
          ? `${result}<br><strong>+${gpSummary.playerRoundPoints}pt</strong><br>${t("totalPoints", { points: game.tournament.playerPoints })} / ${t("nextRoundInfo", { round: game.round + 1 })}`
          : `${result} / ${t("totalTime", { time: formatTime(elapsedMs) })} / ${t("nextRoundInfo", { round: game.round + 1 })}`;
        ui.nextRound.style.display = "inline-block";
        ui.retry.style.display = "none";
      } else if (success) {
        title.textContent = t("finish");
        const result = (game.racePosition || 1) === 1
          ? `<strong>${t("youWin")}</strong>`
          : `<strong>${t("yourPos", { pos: game.racePosition || 1, field: fieldSize })}</strong>`;
        const finalStandings = raceStandings(true, elapsedMs / 1000);
        renderResultWinner((gpSummary?.roundRows || [])[0] || finalStandings[0]);
        renderResultRows(ui.resultStandings, gpSummary ? t("roundResult") : t("result"), gpSummary?.roundRows || finalStandings, gpSummary ? resultRowHtml : finishRowHtml);
        renderResultRows(ui.resultTotal, t("grandPrix"), gpSummary?.tableRows || [], totalRowHtml);
        renderPodiumAircraft(ui.resultTotal, gpSummary?.tableRows || []);
        ui.resultText.innerHTML = gpSummary
          ? `${result}<br><strong>+${gpSummary.playerRoundPoints}pt</strong><br>${t("totalPoints", { points: game.tournament.playerPoints })}<br>${t("finalTime", { time: formatTime(elapsedMs) })} / ${t("damage", { damage: Math.round(game.damage) })}`
          : `${result} / ${t("totalTime", { time: formatTime(elapsedMs) })} / ${t("damage", { damage: Math.round(game.damage) })}`;
        if (scoreQualifies(recordScore)) {
          ui.nameEntry.style.display = "block";
          ui.retry.style.display = "none";
          ui.nameInput.value = localStorage.getItem("airrace_name") || "";
          ui.nameInput.focus();
        }
      } else {
        title.textContent = reason === "crash" ? t("crash") : t("gameOver");
        renderResultWinner(null);
        renderResultRows(ui.resultStandings, t("roundResult"), gpSummary?.roundRows || [], resultRowHtml);
        renderResultRows(ui.resultTotal, t("grandPrix"), gpSummary?.tableRows || [], totalRowHtml);
        renderPodiumAircraft(ui.resultTotal, gpSummary?.tableRows || []);
        ui.resultText.innerHTML = reason === "crash"
          ? t("groundImpactRound", { round: game.round })
          : t("damageRound", { damage: Math.round(game.damage), round: game.round });
        if (gpSummary) {
          ui.resultText.innerHTML += `<br>${t("lastPlacePoints", { points: gpSummary.playerRoundPoints })}`;
        }
        if (recordRound >= 1 && scoreQualifies(recordScore)) {
          ui.resultText.innerHTML += ` / ${t("recordRound", { round: recordRound })}`;
          ui.nameEntry.style.display = "block";
          ui.retry.style.display = "none";
        }
      }
      ui.result.style.display = "flex";
      ui.result.scrollTop = 0;
      updateButtonHints();
    }

    async function start() {
      await startRound(1);
    }

    async function startMainRace() {
      if (selectedStartRound >= GRAND_PRIX_START_ROUND) {
        const carry = { totalStartMs: performance.now(), tournament: createTournamentState() };
        await startRound(selectedStartRound, carry);
        return;
      }
      await startRound(selectedStartRound);
    }

    document.getElementById("start").addEventListener("click", start);
    document.getElementById("start-main").addEventListener("click", startMainRace);
    document.getElementById("retry").addEventListener("click", reloadTitlePage);
    document.getElementById("next-round").addEventListener("click", () => {
      if (!game) return;
      const carry = { totalStartMs: game.totalStartMs, tournament: game.tournament };
      startRound(game.round + 1, carry);
    });
    document.getElementById("save-score").addEventListener("click", () => {
      if (!game) return;
      const elapsedMs = performance.now() - game.totalStartMs;
      const recordRound = game.gateIndex >= game.gates.length && game.round >= TOTAL_ROUNDS ? TOTAL_ROUNDS : Math.max(0, game.round - 1);
      if (recordRound < 1) {
        reloadTitlePage();
        return;
      }
      const name = (ui.nameInput.value || "PILOT").trim().slice(0, 12) || "PILOT";
      localStorage.setItem("airrace_name", name);
      fetch("/api/airrace/scores", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name, score: encodeScore(recordRound, elapsedMs) }),
      }).finally(reloadTitlePage);
    });
    ui.titleNameInput?.addEventListener("input", () => {
      const name = (ui.titleNameInput.value || "").slice(0, 12);
      ui.titleNameInput.value = name;
      localStorage.setItem("airrace_name", name.trim());
      if (ui.nameInput) ui.nameInput.value = name;
    });
    ui.planePrev.addEventListener("click", () => cycleAircraft(-1));
    ui.planeNext.addEventListener("click", () => cycleAircraft(1));
    ui.fieldMode.addEventListener("click", cycleFieldMode);
    ui.roundMode.addEventListener("click", cycleStartRound);

    document.addEventListener("gesturestart", (ev) => ev.preventDefault());
    document.addEventListener("dblclick", (ev) => ev.preventDefault());
    document.addEventListener("contextmenu", (ev) => ev.preventDefault());

    function setControlMode(nextMode) {
      controlMode = nextMode;
      ui.modeStick.classList.toggle("active", controlMode === "stick");
      ui.modeTilt.classList.toggle("active", controlMode === "tilt");
      ui.modeGamepad.classList.toggle("active", controlMode === "gamepad");
      tiltBase = null;
      tiltFiltered = { beta: tilt.beta, gamma: tilt.gamma };
      if (controlMode !== "gamepad") {
        input.boost = false;
        input.throttle = 0;
      }
      updateGamepadStatus();
      updateButtonHints();
    }

    function applyTiltInput() {
      if (controlMode !== "tilt") return;
      if (!tiltBase) {
        tiltBase = { beta: tilt.beta, gamma: tilt.gamma };
        tiltFiltered = { beta: tilt.beta, gamma: tilt.gamma };
      }
      tiltFiltered.beta += (tilt.beta - tiltFiltered.beta) * 0.08;
      tiltFiltered.gamma += (tilt.gamma - tiltFiltered.gamma) * 0.08;
      const pitch = tiltFiltered.beta - tiltBase.beta;
      const roll = tiltFiltered.gamma - tiltBase.gamma;
      const pitchAmount = Math.min(1, Math.abs(pitch) / 38);
      const rollDamping = 1 - pitchAmount * 0.65;
      input.x = tiltAxis((roll * rollDamping) / 66);
      input.y = tiltAxis(pitch / 70);
    }

    function tiltAxis(value) {
      const deadZone = 0.08;
      if (Math.abs(value) < deadZone) return 0;
      const sign = Math.sign(value);
      const scaled = (Math.abs(value) - deadZone) / (1 - deadZone);
      return sign * Math.max(0, Math.min(0.72, scaled));
    }

    async function enableTiltMode() {
      if (typeof DeviceOrientationEvent !== "undefined" && typeof DeviceOrientationEvent.requestPermission === "function") {
        const result = await DeviceOrientationEvent.requestPermission();
        if (result !== "granted") return;
      }
      setControlMode("tilt");
    }

    function axisWithDeadzone(value, deadZone = 0.14) {
      if (Math.abs(value) < deadZone) return 0;
      const sign = Math.sign(value);
      const scaled = (Math.abs(value) - deadZone) / (1 - deadZone);
      return sign * Math.min(1, Math.max(0, scaled));
    }

    function getActiveGamepad() {
      const pads = navigator.getGamepads ? navigator.getGamepads() : [];
      if (activeGamepadIndex !== null && pads[activeGamepadIndex]) {
        return pads[activeGamepadIndex];
      }
      return Array.from(pads).find(Boolean) || null;
    }

    function updateGamepadStatus() {
      if (!ui.gamepadStatus) return;
      const pad = getActiveGamepad();
      if (pad) {
        gamepadName = pad.id || "Gamepad";
        ui.gamepadStatus.textContent = `GAMEPAD: ${gamepadName}`;
      } else {
        ui.gamepadStatus.textContent = t("gamepadGuide");
      }
      updateButtonHints();
    }

    function useGamepadHints() {
      return controlMode === "gamepad" && !!getActiveGamepad();
    }

    function updateButtonHints() {
      const showHints = useGamepadHints();
      ui.start.textContent = showHints ? `× ${buttonLabels.start}` : buttonLabels.start;
      ui.startMain.textContent = showHints ? `□ ${buttonLabels.startMain}` : buttonLabels.startMain;
      ui.saveScore.textContent = showHints ? `× ${buttonLabels.saveScore}` : buttonLabels.saveScore;
      ui.nextRound.textContent = showHints ? `× ${buttonLabels.nextRound}` : buttonLabels.nextRound;
      ui.retry.textContent = showHints ? `○ ${buttonLabels.retry}` : buttonLabels.retry;
    }

    function applyGamepadInput() {
      if (controlMode !== "gamepad") return;
      const pad = getActiveGamepad();
      if (!pad) {
        input.x = 0;
        input.y = 0;
        input.boost = false;
        input.throttle = 0;
        updateGamepadStatus();
        return;
      }
      activeGamepadIndex = pad.index;
      input.x = axisWithDeadzone(pad.axes[0] || 0);
      input.y = axisWithDeadzone(pad.axes[1] || 0);
      const cross = pad.buttons[0]?.pressed || false;
      const r1 = pad.buttons[5]?.pressed || false;
      const r2Value = pad.buttons[7]?.value || 0;
      input.throttle = Math.max(cross || r1 ? 1 : 0, r2Value);
      input.boost = input.throttle > 0.02;
    }

    function isVisible(el) {
      return !!el && getComputedStyle(el).display !== "none";
    }

    function gamepadPressedOnce(pad, index) {
      const pressed = pad.buttons[index]?.pressed || false;
      const wasPressed = gamepadPrevButtons[index] || false;
      gamepadPrevButtons[index] = pressed;
      return pressed && !wasPressed;
    }

    function pollGamepadUi() {
      const pad = getActiveGamepad();
      if (!pad || controlMode !== "gamepad") {
        requestAnimationFrame(pollGamepadUi);
        return;
      }

      // UI操作は「押した瞬間」だけ使う。BOOST押しっぱなしからの誤爆を避ける。
      const crossOnce = gamepadPressedOnce(pad, 0);
      const circleOnce = gamepadPressedOnce(pad, 1);
      const optionsOnce = gamepadPressedOnce(pad, 9);
      const leftOnce = gamepadPressedOnce(pad, 14);
      const rightOnce = gamepadPressedOnce(pad, 15);

      if (isVisible(ui.title)) {
        if (leftOnce) cycleAircraft(-1);
        if (rightOnce) cycleAircraft(1);
      }

      if (crossOnce) {
        if (isVisible(ui.title)) {
          start();
        } else if (isVisible(ui.nextRound)) {
          ui.nextRound.click();
        } else if (isVisible(ui.nameEntry)) {
          ui.saveScore.click();
        } else if (isVisible(ui.retry)) {
          ui.retry.click();
        }
      }

      if (gamepadPressedOnce(pad, 3) && isVisible(ui.title)) {
        startMainRace();
      }

      if (circleOnce || optionsOnce) {
        if (isVisible(ui.result) || isVisible(ui.title)) {
          showTitle(true);
        }
      }

      requestAnimationFrame(pollGamepadUi);
    }

    window.addEventListener("deviceorientation", (ev) => {
      tilt.beta = Number(ev.beta || 0);
      tilt.gamma = Number(ev.gamma || 0);
    });

    if ("DeviceOrientationEvent" in window || "getGamepads" in navigator) {
      ui.controlMode.style.display = "flex";
    }
    ui.modeStick.addEventListener("click", () => setControlMode("stick"));
    ui.modeTilt.addEventListener("click", enableTiltMode);
    ui.modeGamepad.addEventListener("click", () => {
      setControlMode("gamepad");
      updateGamepadStatus();
    });

    window.addEventListener("gamepadconnected", (ev) => {
      activeGamepadIndex = ev.gamepad.index;
      gamepadName = ev.gamepad.id || "Gamepad";
      gamepadPrevButtons = [];
      updateGamepadStatus();
      if (controlMode === "stick") {
        setControlMode("gamepad");
      }
    });

    window.addEventListener("gamepaddisconnected", (ev) => {
      if (activeGamepadIndex === ev.gamepad.index) {
        activeGamepadIndex = null;
        gamepadPrevButtons = [];
        input.boost = false;
        input.throttle = 0;
      }
      updateGamepadStatus();
    });

    document.addEventListener("keydown", (ev) => {
      if (isVisible(ui.title)) {
        if (ev.key === "ArrowLeft" || ev.key.toLowerCase() === "a") {
          ev.preventDefault();
          cycleAircraft(-1);
          return;
        }
        if (ev.key === "ArrowRight" || ev.key.toLowerCase() === "d") {
          ev.preventDefault();
          cycleAircraft(1);
          return;
        }
        if (ev.key === "Enter" || ev.code === "Space") {
          ev.preventDefault();
          start();
          return;
        }
        if (ev.key === "r" || ev.key === "R") {
          ev.preventDefault();
          cycleStartRound();
          return;
        }
        if (ev.key === "3") {
          ev.preventDefault();
          startMainRace();
          return;
        }
      }
      if (ev.key === "ArrowLeft" || ev.key.toLowerCase() === "a") keyboardTarget.x = -0.72;
      if (ev.key === "ArrowRight" || ev.key.toLowerCase() === "d") keyboardTarget.x = 0.72;
      if (ev.key === "ArrowUp" || ev.key.toLowerCase() === "w") keyboardTarget.y = -0.86;
      if (ev.key === "ArrowDown" || ev.key.toLowerCase() === "s") keyboardTarget.y = 0.86;
      if (ev.code === "Space") {
        input.boost = true;
        input.throttle = 1;
      }
    });

    document.addEventListener("keyup", (ev) => {
      if (["ArrowLeft", "ArrowRight", "a", "d", "A", "D"].includes(ev.key)) keyboardTarget.x = 0;
      if (["ArrowUp", "ArrowDown", "w", "s", "W", "S"].includes(ev.key)) keyboardTarget.y = 0;
      if (ev.code === "Space") {
        input.boost = false;
        input.throttle = 0;
      }
    });

    document.getElementById("boost").addEventListener("pointerdown", (ev) => {
      ev.preventDefault();
      input.boost = true;
      input.throttle = 1;
    });
    document.getElementById("boost").addEventListener("pointerup", () => {
      input.boost = false;
      input.throttle = 0;
    });
    document.getElementById("boost").addEventListener("pointercancel", () => {
      input.boost = false;
      input.throttle = 0;
    });

    let stickPointer = null;
    ui.stick.addEventListener("pointerdown", (ev) => {
      stickPointer = ev.pointerId;
      ui.stick.setPointerCapture(stickPointer);
      updateStick(ev);
    });
    ui.stick.addEventListener("pointermove", (ev) => {
      if (ev.pointerId === stickPointer) updateStick(ev);
    });
    ui.stick.addEventListener("pointerup", resetStick);
    ui.stick.addEventListener("pointercancel", resetStick);

    function updateStick(ev) {
      const rect = ui.stick.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const dx = ev.clientX - cx;
      const dy = ev.clientY - cy;
      const len = Math.min(38, Math.hypot(dx, dy));
      const angle = Math.atan2(dy, dx);
      const kx = Math.cos(angle) * len;
      const ky = Math.sin(angle) * len;
      input.x = kx / 38;
      input.y = ky / 38;
      ui.knob.style.transform = `translate(${kx}px, ${ky}px)`;
    }

    function resetStick() {
      stickPointer = null;
      input.x = 0;
      input.y = 0;
      ui.knob.style.transform = "translate(0, 0)";
    }

    window.addEventListener("resize", () => {
      resizeCanvas();
      if (!game) drawSplash();
    });
    window.addEventListener("orientationchange", () => {
      setTimeout(() => {
        resizeCanvas();
        if (!game) drawSplash();
      }, 250);
    });

    resizeCanvas();
    applyLanguage();
    updateButtonHints();
    pollGamepadUi();
    showTitle(true);

    function drawSplash() {
      ctx.fillStyle = "#071827";
      ctx.fillRect(0, 0, W, H);
      ctx.strokeStyle = "rgba(103,232,249,.45)";
      for (let i = -6; i <= 6; i += 1) {
        ctx.beginPath();
        ctx.moveTo(centerX, horizon);
        ctx.lineTo(centerX + i * 55, H);
        ctx.stroke();
      }
      ctx.fillStyle = "#67e8f9";
      ctx.font = "14px DotGothic16";
      ctx.textAlign = "center";
      ctx.fillText("VECTOR SCAN AIR COURSE", centerX, horizon + 15);
    }
