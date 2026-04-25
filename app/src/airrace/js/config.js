    const canvas = document.getElementById("c");
    const ctx = canvas.getContext("2d");
    let W = canvas.width;
    let H = canvas.height;
    let horizon = 230;
    let centerX = W / 2;
    let centerY = 255;
    let worldWidth = 420;
    const minAlt = 0;
    const maxAlt = 1000;
    const cruiseSpeed = 340;
    const maxBoostSpeed = 1500;
    const rollVisual = 0.88;
    const groundY = -110;
    const gateWorldRadius = 86;

    function resizeCanvas() {
      const rect = canvas.getBoundingClientRect();
      const cssW = Math.max(320, Math.round(rect.width));
      const cssH = Math.max(320, Math.round(rect.height));
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      if (canvas.width !== Math.round(cssW * dpr) || canvas.height !== Math.round(cssH * dpr)) {
        canvas.width = Math.round(cssW * dpr);
        canvas.height = Math.round(cssH * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      W = cssW;
      H = cssH;
      centerX = W / 2;
      centerY = H * 0.51;
      horizon = H * 0.46;
      worldWidth = Math.max(420, W * 1.12);
    }

    const ui = {
      speed: document.getElementById("speed"),
      alt: document.getElementById("alt"),
      time: document.getElementById("time"),
      message: document.getElementById("message"),
      title: document.getElementById("title"),
      result: document.getElementById("result"),
      resultText: document.getElementById("result-text"),
      resultWinner: document.getElementById("result-winner"),
      resultPlanePreview: document.getElementById("result-plane-preview"),
      resultWinnerName: document.getElementById("result-winner-name"),
      resultWinnerAircraft: document.getElementById("result-winner-aircraft"),
      resultStandings: document.getElementById("result-standings"),
      resultTotal: document.getElementById("result-total"),
      nameEntry: document.getElementById("name-entry"),
      nameInput: document.getElementById("name-input"),
      titleNameInput: document.getElementById("title-name"),
      nextRound: document.getElementById("next-round"),
      retry: document.getElementById("retry"),
      start: document.getElementById("start"),
      startMain: document.getElementById("start-main"),
      saveScore: document.getElementById("save-score"),
      controlMode: document.getElementById("control-mode"),
      modeStick: document.getElementById("mode-stick"),
      modeTilt: document.getElementById("mode-tilt"),
      modeGamepad: document.getElementById("mode-gamepad"),
      gamepadStatus: document.getElementById("gamepad-status"),
      planeSelect: document.getElementById("plane-select"),
      planePrev: document.getElementById("plane-prev"),
      planeNext: document.getElementById("plane-next"),
      fieldMode: document.getElementById("field-mode"),
      roundMode: document.getElementById("round-mode"),
      planeDisplay: document.getElementById("plane-display"),
      planePreview: document.getElementById("plane-preview"),
      planeName: document.getElementById("plane-name"),
      planeSummary: document.getElementById("plane-summary"),
      loading: document.getElementById("loading"),
      loadingLabel: document.getElementById("loading-label"),
      loadingFill: document.getElementById("loading-fill"),
      loadingMeta: document.getElementById("loading-meta"),
      knob: document.getElementById("knob"),
      stick: document.getElementById("stick"),
      guide: document.getElementById("guide"),
    };

    const input = {
      x: 0,
      y: 0,
      boost: false,
      throttle: 0,
    };

    let game = null;
    let raf = null;
    let audio = null;
    let controlMode = "stick";
    let selectedAircraftId = localStorage.getItem("airrace_aircraft") || "skylancer";
    let selectedFieldModeId = localStorage.getItem("airrace_field_mode") || "normal";
    let selectedStartRound = Math.max(1, Math.min(10, Number(localStorage.getItem("airrace_start_round") || 3) || 3));
    let tiltBase = null;
    let tilt = { beta: 0, gamma: 0 };
    let tiltFiltered = { beta: 0, gamma: 0 };
    let activeGamepadIndex = null;
    let gamepadName = "";
    let gamepadPrevButtons = [];
    const keyboardTarget = { x: 0, y: 0 };
    window.airraceRoundCache = {};
    let loadingToken = 0;

    const buttonLabels = {
      start: "ROUND 1 START",
      startMain: "ROUND 3 START",
      saveScore: "SAVE",
      nextRound: "NEXT ROUND",
      retry: "TITLE",
    };

    const AIRCRAFTS = [
      {
        id: "skylancer",
        name: "スカイランサー号",
        summary: "平均的な優等生。初見コースの基準機。",
        speedMul: 1.0,
        boostMul: 1.0,
        turnMul: 1.0,
        climbMul: 1.0,
        color: "rgba(125, 211, 252, 0.78)",
        stroke: "#e0f2fe",
        shape: "standard",
      },
      {
        id: "thunderbolt",
        name: "サンダーボルト号",
        summary: "きびきび旋回。テクニカル向け。",
        speedMul: 0.94,
        boostMul: 0.94,
        turnMul: 1.24,
        climbMul: 1.08,
        color: "rgba(250, 204, 21, 0.78)",
        stroke: "#fef3c7",
        shape: "broad",
      },
      {
        id: "shootingstar",
        name: "シューティングスター号",
        summary: "高速番長。直線と大カーブで強い。",
        speedMul: 1.08,
        boostMul: 1.16,
        turnMul: 0.82,
        climbMul: 0.94,
        color: "rgba(248, 113, 113, 0.80)",
        stroke: "#fee2e2",
        shape: "dart",
      },
      {
        id: "spiralfang",
        name: "スパイラルファング号",
        summary: "ピーキーな軽量機。反応最速。",
        speedMul: 0.98,
        boostMul: 0.98,
        turnMul: 1.32,
        climbMul: 1.14,
        color: "rgba(192, 132, 252, 0.82)",
        stroke: "#f3e8ff",
        shape: "fang",
      },
      {
        id: "ironhawk",
        name: "アイアンホーク号",
        summary: "重いけど安定。崩れにくい。",
        speedMul: 1.03,
        boostMul: 1.08,
        turnMul: 0.9,
        climbMul: 0.92,
        color: "rgba(74, 222, 128, 0.76)",
        stroke: "#dcfce7",
        shape: "heavy",
      },
    ];

    const TOTAL_ROUNDS = 10;
    const GRAND_PRIX_START_ROUND = 3;
    const USE_GHOST_RIVALS = true;
    const POINT_TABLE = [10, 8, 6, 4, 2];
    const FIELD_MODES = [
      { id: "normal", label: "FIELD NORMAL", round2Size: 3, gpSize: 5 },
      { id: "test8", label: "FIELD TEST 8", round2Size: 8, gpSize: 8 },
      { id: "test12", label: "FIELD TEST 12", round2Size: 12, gpSize: 12 },
      { id: "test16", label: "FIELD TEST 16", round2Size: 16, gpSize: 16 },
      { id: "test24", label: "FIELD TEST 24", round2Size: 24, gpSize: 24 },
    ];
    const RIVAL_ARCHETYPES = [
      {
        id: "blaze",
        name: "BLAZE FALCON",
        aircraftId: "shootingstar",
        style: "speed",
        laneOffset: -156,
        label: "SPEED",
        paceByRound: { 3: 0.93, 4: 1.00, 5: 0.91, 6: 1.02, 7: 1.01, 8: 0.94, 9: 0.90, 10: 1.03 },
        wave: 22,
      },
      {
        id: "ciel",
        name: "CIEL NOVA",
        aircraftId: "thunderbolt",
        style: "line",
        laneOffset: -52,
        label: "LINE",
        paceByRound: { 3: 0.99, 4: 0.94, 5: 1.00, 6: 0.95, 7: 0.96, 8: 1.01, 9: 1.03, 10: 0.93 },
        wave: 10,
      },
      {
        id: "gaia",
        name: "GAIA PHOENIX",
        aircraftId: "ironhawk",
        style: "steady",
        laneOffset: 58,
        label: "POWER",
        paceByRound: { 3: 1.10, 4: 1.08, 5: 1.05, 6: 1.11, 7: 1.07, 8: 1.12, 9: 1.09, 10: 1.04 },
        wave: 4,
      },
      {
        id: "viper",
        name: "VIPER SWING",
        aircraftId: "spiralfang",
        style: "line",
        laneOffset: 162,
        label: "AGILE",
        paceByRound: { 3: 1.05, 4: 0.98, 5: 1.02, 6: 1.00, 7: 0.97, 8: 0.99, 9: 1.01, 10: 0.95 },
        wave: 18,
      },
      {
        id: "nova",
        name: "NOVA DART",
        aircraftId: "skylancer",
        style: "steady",
        laneOffset: 0,
        label: "BALANCE",
        paceByRound: { 3: 1.02, 4: 1.01, 5: 0.99, 6: 1.00, 7: 1.02, 8: 1.00, 9: 0.98, 10: 0.97 },
        wave: 7,
      },
      {
        id: "orca",
        name: "ORCA CRUISE",
        aircraftId: "ironhawk",
        style: "steady",
        laneOffset: 0,
        label: "HEAVY",
        paceByRound: { 3: 1.13, 4: 1.10, 5: 1.09, 6: 1.15, 7: 1.12, 8: 1.14, 9: 1.11, 10: 1.08 },
        wave: 3,
      },
    ];

    function pointsForPosition(position) {
      return POINT_TABLE[Math.max(0, position - 1)] || 0;
    }

    function currentFieldSize() {
      return 1 + ((game?.rivals?.length) || 0);
    }

    function selectedFieldMode() {
      return FIELD_MODES.find((mode) => mode.id === selectedFieldModeId) || FIELD_MODES[0];
    }

    function cycleFieldMode() {
      const index = Math.max(0, FIELD_MODES.findIndex((mode) => mode.id === selectedFieldModeId));
      const next = FIELD_MODES[(index + 1) % FIELD_MODES.length];
      selectedFieldModeId = next.id;
      localStorage.setItem("airrace_field_mode", selectedFieldModeId);
      renderFieldMode();
    }

    function renderFieldMode() {
      if (!ui.fieldMode) return;
      const mode = selectedFieldMode();
      ui.fieldMode.textContent = mode.label;
    }

    function cycleStartRound() {
      selectedStartRound = selectedStartRound >= TOTAL_ROUNDS ? 1 : selectedStartRound + 1;
      localStorage.setItem("airrace_start_round", String(selectedStartRound));
      renderStartRound();
    }

    function renderStartRound() {
      if (selectedStartRound > TOTAL_ROUNDS) selectedStartRound = TOTAL_ROUNDS;
      buttonLabels.startMain = `ROUND ${selectedStartRound} START`;
      if (ui.roundMode) ui.roundMode.textContent = `START ROUND ${selectedStartRound}`;
      updateButtonHints();
    }

    function aircraftById(id) {
      return AIRCRAFTS.find((plane) => plane.id === id) || AIRCRAFTS[0];
    }

    function buildLaneOffsets(count) {
      const spacing = count >= 14 ? 38 : count >= 10 ? 46 : count >= 6 ? 60 : 104;
      const offsets = [];
      for (let i = 0; i < count; i += 1) {
        let offset = (i - (count - 1) / 2) * spacing;
        if (Math.abs(offset) < spacing * 0.45) offset += offset >= 0 ? spacing * 0.6 : -spacing * 0.6;
        offsets.push(Math.round(offset));
      }
      return offsets;
    }

    function buildRivalProfiles(count) {
      const offsets = buildLaneOffsets(count);
      return Array.from({ length: count }, (_, index) => {
        const base = RIVAL_ARCHETYPES[index % RIVAL_ARCHETYPES.length];
        const tier = Math.floor(index / RIVAL_ARCHETYPES.length);
        const paceShift = tier * 0.025 + (index >= count - 2 ? 0.03 : 0);
        const suffix = tier > 0 ? ` ${tier + 1}` : "";
        return {
          ...base,
          id: `${base.id}_${index}`,
          name: `${base.name}${suffix}`,
          laneOffset: offsets[index],
          aircraft: aircraftById(base.aircraftId),
          points: 0,
          paceByRound: Object.fromEntries(
            Object.entries(base.paceByRound || {}).map(([round, pace]) => [round, pace + paceShift])
          ),
          wave: (base.wave || 0) + tier * 2,
        };
      });
    }

    function createTournamentState() {
      const mode = selectedFieldMode();
      return {
        playerPoints: 0,
        rounds: [],
        rivals: buildRivalProfiles(Math.max(0, mode.gpSize - 1)),
      };
    }

    function tournamentStatusText(tournament) {
      if (!tournament) return "";
      const rows = [
        { name: "YOU", points: tournament.playerPoints || 0 },
        ...tournament.rivals.map((rival) => ({ name: rival.name, points: rival.points || 0 })),
      ].sort((a, b) => b.points - a.points);
      return rows.map((row, index) => `${index + 1}.${row.name} ${row.points}pt`).join(" / ");
    }

    function setLoadingState(visible, label = "NOW LOADING", progress = 0, meta = "") {
      if (!ui.loading) return;
      ui.loading.style.display = visible ? "flex" : "none";
      if (ui.loadingLabel) ui.loadingLabel.textContent = label;
      if (ui.loadingFill) ui.loadingFill.style.width = `${Math.max(0, Math.min(100, progress * 100))}%`;
      if (ui.loadingMeta) ui.loadingMeta.textContent = meta;
      if (ui.start) ui.start.disabled = visible;
      if (ui.nextRound) ui.nextRound.disabled = visible;
    }

    function currentRoundCache(round) {
      return window.airraceRoundCache?.[round] || null;
    }

    async function fetchRoundBundle(round, onProgress = () => {}, forceRefresh = false) {
      const cached = !forceRefresh ? currentRoundCache(round) : null;
      if (cached) return cached;
      onProgress(0.18, "course json");
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), 8000);
      const res = await fetch(`/api/airrace/round/${round}?ts=${Date.now()}`, {
        cache: "no-store",
        signal: controller.signal,
      }).finally(() => clearTimeout(timer));
      if (!res.ok) throw new Error(`round fetch failed: ${round}`);
      onProgress(0.56, "ghost data");
      const data = await res.json();
      const bundle = {
        round,
        course: data.course || {},
        ghosts: Array.isArray(data.ghosts) ? data.ghosts : [],
      };
      window.airraceRoundCache[round] = bundle;
      onProgress(0.82, "scan build");
      return bundle;
    }

    async function preloadRound(round) {
      if (round > TOTAL_ROUNDS || currentRoundCache(round)) return;
      try {
        await fetchRoundBundle(round);
      } catch {}
    }

    function roundDifficulty(round) {
      return {
        spacingScale: round === 1 ? 3.7 : round === 2 ? 3.2 : Math.max(1.35, 2.95 - (round - 1) * 0.42),
        turnScale: round === 1 ? 0.16 : round === 2 ? 0.24 : 0.36 + (round - 1) * 0.16,
        lateralScale: round === 1 ? 0.30 : round === 2 ? 0.38 : 0.45 + (round - 1) * 0.18,
        verticalScale: round === 1 ? 0.10 : round === 2 ? 0.18 : 0.35 + (round - 1) * 0.22,
      };
    }

    function roundBriefing(round) {
      const mode = selectedFieldMode();
      const round2Rivals = Math.max(0, mode.round2Size - 1);
      const gpRivals = Math.max(0, mode.gpSize - 1);
      const briefings = [
        {
          title: "ROUND 1 / 離陸訓練",
          difficulty: 1,
          text: "ライバルなし。大きいゲートを順番にくぐって、まずは離陸と旋回に慣れよう。",
        },
        {
          title: "ROUND 2 / 高度変化",
          difficulty: 2,
          text: `ここから${mode.round2Size}機レース。${round2Rivals}機を相手に、ゆるい高低差と順位争いを覚えよう。`,
        },
        {
          title: "ROUND 3 / USA DESERT OPENING",
          difficulty: 3,
          text: `アメリカ砂漠ラウンド。長い直線と赤土キャニオンで World Grand Prix 開幕だ。${mode.gpSize}機で一気に出る。`,
        },
        {
          title: "ROUND 4 / FRANCE RIVER CIRCUIT",
          difficulty: 4,
          text: `フランス河川都市ラウンド。中速カーブをつないで、ライン取りで差を作るテクニカル戦。`,
        },
        {
          title: "ROUND 5 / ITALY COASTAL SPRINT",
          difficulty: 5,
          text: "イタリア海岸ラウンド。海沿いのハイスピード区間で、BOOSTを気持ちよく使える高速戦。",
        },
        {
          title: "ROUND 6 / NETHERLANDS HARBOR WIND",
          difficulty: 6,
          text: "オランダ港湾ラウンド。低い空と細かな向き変えで、落ち着いた操縦が効く。",
        },
        {
          title: "ROUND 7 / GERMANY RHINE INDUSTRIAL",
          difficulty: 7,
          text: "ドイツ工業地帯ラウンド。中速の切り返しと重めの流れで、安定感のある機体が光る。",
        },
        {
          title: "ROUND 8 / CHINA MEGACITY RING",
          difficulty: 8,
          text: "中国メガシティラウンド。高層都市を大きく回る高速外周で、視界も速度感も一気に上がる。",
        },
        {
          title: "ROUND 9 / UAE SKY DUNE RUSH",
          difficulty: 9,
          text: "UAE 砂丘ラウンド。高速直線から一気に曲げる見せ場コース。決勝前に差を動かす最後の山場。",
        },
        {
          title: "ROUND 10 / JAPAN GRAND FINAL",
          difficulty: 10,
          text: "最終戦 JAPAN。テクニカル区間と高速区間の両方をまとめて取り切って、総合優勝を決めろ。",
        },
      ];
      return briefings[Math.max(0, Math.min(briefings.length - 1, round - 1))];
    }
