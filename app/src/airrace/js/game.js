    function resetGame(round = 1, carry = null, bundle = null) {
      const course = makeCourse(round, bundle?.course || null);
      const aircraft = selectedAircraft();
      startAudio();
      const now = performance.now();
      const tournament = carry?.tournament || createTournamentState();
      game = {
        running: true,
        finished: false,
        aircraft,
        round,
        tournament,
        totalStartMs: carry?.totalStartMs || now,
        roundStartMs: now,
        lastMs: now,
        distance: 0,
        phase: "countdown",
        countdownStartMs: now,
        takeoffAssistUntil: now + 5200,
        speed: 0,
        altitude: 24,
        planeX: 0,
        planeZ: -520,
        heading: 0,
        roll: 0,
        pitchVisual: 0,
        damage: 0,
        gateIndex: 0,
        messageUntil: 0,
        roundGhosts: bundle?.ghosts || [],
        roundBundle: bundle || null,
        ghostSamples: [],
        lastGhostSampleMs: -999,
        racePosition: 1,
        groundHitAt: null,
        ...course,
      };
      game.rivals = makeRivals(round, game.coursePath);
      ui.title.style.display = "none";
      ui.result.style.display = "none";
      ui.nameEntry.style.display = "none";
      ui.nextRound.style.display = "none";
      ui.retry.style.display = "none";
      ui.guide.style.display = "none";
      ui.message.textContent = "";
      ui.message.style.display = "none";
    }

    function setMessage(text, ms = 1200) {
      ui.message.textContent = text;
      ui.message.style.display = text ? "block" : "none";
      if (game) game.messageUntil = performance.now() + ms;
    }

    function syncPilotNameInput() {
      const stored = (localStorage.getItem("airrace_name") || "").trim();
      if (ui.titleNameInput && ui.titleNameInput.value !== stored) {
        ui.titleNameInput.value = stored;
      }
      if (ui.nameInput && ui.nameInput.value !== stored) {
        ui.nameInput.value = stored;
      }
    }

    function reloadTitlePage() {
      window.location.href = "/airrace";
    }

    function raceStandings(success, elapsedSeconds = null) {
      const playerTime = success ? (elapsedSeconds ?? ((performance.now() - game.totalStartMs) / 1000)) : Number.POSITIVE_INFINITY;
      const entries = [{
        kind: "player",
        id: "player",
        name: t("you"),
        aircraft: game.aircraft,
        finalTime: playerTime,
        dnf: !success,
      }];
      for (const rival of game.rivals || []) {
        const dnf = rival.status === "dnf";
        entries.push({
          kind: "rival",
          id: rival.profileId || rival.name,
          name: rival.name,
          aircraft: rival.aircraft,
          finalTime: dnf ? Number.POSITIVE_INFINITY : (rival.totalTime || Number.POSITIVE_INFINITY),
          dnf,
        });
      }
      entries.sort((a, b) => {
        if (a.dnf !== b.dnf) return a.dnf ? 1 : -1;
        return a.finalTime - b.finalTime;
      });
      return entries.map((entry, index) => ({
        ...entry,
        position: index + 1,
      }));
    }

    function applyTournamentPoints(success, elapsedSeconds = null) {
      if (!game?.tournament || game.round < GRAND_PRIX_START_ROUND) return null;
      const standings = raceStandings(success, elapsedSeconds);
      const playerEntry = standings.find((entry) => entry.kind === "player");
      const playerRoundPoints = pointsForPosition(playerEntry?.position || currentFieldSize());
      game.tournament.playerPoints += playerRoundPoints;
      for (const entry of standings) {
        if (entry.kind !== "rival") continue;
        const rival = game.tournament.rivals.find((item) => item.id === entry.id);
        if (rival) rival.points += pointsForPosition(entry.position);
      }
      game.tournament.rounds.push({
        round: game.round,
        standings,
      });
      const roundRows = standings.map((entry) => ({
        position: entry.position,
        name: entry.kind === "player" ? t("you") : entry.name,
        kind: entry.kind,
        aircraft: entry.aircraft,
        points: pointsForPosition(entry.position),
        dnf: entry.dnf,
      }));
      const tableRows = [
        { name: t("you"), points: game.tournament.playerPoints || 0, kind: "player", aircraft: game.aircraft },
        ...game.tournament.rivals.map((rival) => ({
          name: rival.name,
          points: rival.points || 0,
          kind: "rival",
          aircraft: rival.aircraft,
        })),
      ].sort((a, b) => b.points - a.points);
      return {
        standings,
        playerPosition: playerEntry?.position || currentFieldSize(),
        playerRoundPoints,
        roundRows,
        tableRows,
        table: tournamentStatusText(game.tournament),
      };
    }

    function encodeScore(round, ms) {
      return round * 10_000_000 + Math.max(0, 9_999_999 - Math.round(ms));
    }

    function decodeScore(score) {
      const round = Math.floor(score / 10_000_000);
      const ms = 9_999_999 - (score % 10_000_000);
      return { round, ms };
    }

    async function fetchScores() {
      const res = await fetch("/api/airrace/scores");
      if (!res.ok) return { scores: [], min_score: 0 };
      return res.json();
    }

    function scoreQualifies(score) {
      const scores = window.airraceScores || [];
      return scores.length < 5 || score > Number(scores[scores.length - 1]?.score || 0);
    }

    async function renderRankings() {
      const data = await fetchScores();
      window.airraceScores = data.scores || [];
    }

    function formatRank(score) {
      const decoded = decodeScore(Number(score || 0));
      return `round${decoded.round} ${formatTime(decoded.ms)}`;
    }

    function formatTime(ms) {
      return (ms / 1000).toFixed(2);
    }

    function showTitle(force = false) {
      if (!force && game?.running) return;
      stopAudio();
      cancelAnimationFrame(raf);
      game = null;
      window.airraceRoundCache = {};
      setLoadingState(false);
      ui.title.style.display = "flex";
      ui.result.style.display = "none";
      ui.guide.style.display = "none";
      ui.nameEntry.style.display = "none";
      ui.nextRound.style.display = "none";
      ui.retry.style.display = "none";
      renderPlaneSelect();
      renderRankings();
      syncPilotNameInput();
      updateButtonHints();
      drawSplash();
      ui.message.textContent = t("defaultMessage");
      ui.message.style.display = "block";
    }

    async function startRound(round, carry = null) {
      const token = ++loadingToken;
      const label = round === 1 ? t("loadingStart") : t("loadingRound", { round });
      setLoadingState(true, label, 0.08, "prepare request");
      try {
        const bundle = await fetchRoundBundle(round, (progress, meta) => {
          if (token !== loadingToken) return;
          setLoadingState(true, label, progress, meta);
        }, true);
        if (token !== loadingToken) return;
        setLoadingState(true, label, 0.94, "cockpit sync");
        resetGame(round, carry, bundle);
        raf = requestAnimationFrame(frame);
        setLoadingState(false);
        preloadRound(round + 1);
      } catch (err) {
        console.error(err);
        setLoadingState(false);
        showTitle(true);
        setMessage(t("loadFailed", { round }), 1800);
      }
    }

    function startAudio() {
      if (audio?.ac?.state === "suspended") {
        audio.ac.resume();
        return;
      }
      if (audio) return;
      const ac = new (window.AudioContext || window.webkitAudioContext)();
      const prop = ac.createOscillator();
      const wobble = ac.createOscillator();
      const propGain = ac.createGain();
      const wobbleGain = ac.createGain();
      prop.type = "sawtooth";
      wobble.type = "square";
      prop.frequency.value = 84;
      wobble.frequency.value = 31;
      propGain.gain.value = 0.025;
      wobbleGain.gain.value = 0.0;
      prop.connect(propGain).connect(ac.destination);
      wobble.connect(wobbleGain).connect(ac.destination);
      prop.start();
      wobble.start();
      audio = { ac, prop, wobble, propGain, wobbleGain };
    }

    function stopAudio() {
      if (!audio) return;
      const t = audio.ac.currentTime;
      audio.propGain.gain.cancelScheduledValues(t);
      audio.wobbleGain.gain.cancelScheduledValues(t);
      audio.propGain.gain.setTargetAtTime(0.0, t, 0.04);
      audio.wobbleGain.gain.setTargetAtTime(0.0, t, 0.04);
    }

    function updateAudio() {
      if (!audio) return;
      const t = audio.ac.currentTime;
      audio.prop.frequency.setTargetAtTime(70 + game.speed * 0.10, t, 0.05);
      audio.propGain.gain.setTargetAtTime(0.024 + input.throttle * 0.025, t, 0.05);
      audio.wobble.frequency.setTargetAtTime(26 + input.throttle * 24, t, 0.04);
      audio.wobbleGain.gain.setTargetAtTime(input.throttle * 0.026, t, 0.04);
    }

    function hitSound(kind) {
      if (!audio) return;
      const ac = audio.ac;
      const osc = ac.createOscillator();
      const gain = ac.createGain();
      osc.connect(gain).connect(ac.destination);
      if (kind === "crash") {
        osc.type = "sawtooth";
        osc.frequency.setValueAtTime(90, ac.currentTime);
        osc.frequency.exponentialRampToValueAtTime(28, ac.currentTime + 0.45);
        gain.gain.setValueAtTime(0.16, ac.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.001, ac.currentTime + 0.55);
        osc.start();
        osc.stop(ac.currentTime + 0.58);
        return;
      }
      osc.type = kind === "gate" ? "triangle" : "square";
      osc.frequency.setValueAtTime(kind === "gate" ? 760 : 130, ac.currentTime);
      gain.gain.setValueAtTime(kind === "gate" ? 0.055 : 0.10, ac.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, ac.currentTime + 0.18);
      osc.start();
      osc.stop(ac.currentTime + 0.2);
    }

    function playerWorldY() {
      return game.altitude - 150;
    }

    function updateCamera(dt) {
      const targetPitch = input.y * 0.62;
      game.pitchVisual += (targetPitch - game.pitchVisual) * Math.min(1, dt * 3.8);
      horizon = H * 0.46 + game.pitchVisual * H * 0.17;
    }
