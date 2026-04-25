    function catmullRomPoint(p0, p1, p2, p3, t) {
      const t2 = t * t;
      const t3 = t2 * t;
      return {
        x: 0.5 * ((2 * p1.x) + (-p0.x + p2.x) * t + (2 * p0.x - 5 * p1.x + 4 * p2.x - p3.x) * t2 + (-p0.x + 3 * p1.x - 3 * p2.x + p3.x) * t3),
        y: 0.5 * ((2 * p1.y) + (-p0.y + p2.y) * t + (2 * p0.y - 5 * p1.y + 4 * p2.y - p3.y) * t2 + (-p0.y + 3 * p1.y - 3 * p2.y + p3.y) * t3),
        z: 0.5 * ((2 * p1.z) + (-p0.z + p2.z) * t + (2 * p0.z - 5 * p1.z + 4 * p2.z - p3.z) * t2 + (-p0.z + 3 * p1.z - 3 * p2.z + p3.z) * t3),
      };
    }

    function buildSmoothCoursePath(points, subdivisions = 12) {
      if (!points || points.length < 2) return points || [];
      const out = [];
      for (let i = 0; i < points.length - 1; i += 1) {
        const p0 = points[Math.max(0, i - 1)];
        const p1 = points[i];
        const p2 = points[i + 1];
        const p3 = points[Math.min(points.length - 1, i + 2)];
        for (let j = 0; j < subdivisions; j += 1) {
          out.push(catmullRomPoint(p0, p1, p2, p3, j / subdivisions));
        }
      }
      out.push({ ...points[points.length - 1] });
      for (let i = 0; i < out.length; i += 1) {
        const prev = out[Math.max(0, i - 1)];
        const next = out[Math.min(out.length - 1, i + 1)];
        out[i].heading = Math.atan2(next.x - prev.x, next.z - prev.z);
      }
      return out;
    }

    function sampleGuideMarkers(path, spacing) {
      if (!path || path.length < 2) return [];
      const markers = [];
      let traveled = 0;
      let target = spacing;
      for (let i = 1; i < path.length; i += 1) {
        const a = path[i - 1];
        const b = path[i];
        const seg = Math.hypot(b.x - a.x, b.y - a.y, b.z - a.z);
        if (seg <= 0.001) continue;
        while (traveled + seg >= target) {
          const f = (target - traveled) / seg;
          markers.push({
            x: a.x + (b.x - a.x) * f,
            y: a.y + (b.y - a.y) * f,
            z: a.z + (b.z - a.z) * f,
            pathIndex: i - 1 + f,
          });
          target += spacing;
        }
        traveled += seg;
      }
      return markers;
    }

    function nearestPathIndex(path, target) {
      if (!path || path.length === 0) return 0;
      let bestIndex = 0;
      let bestDist = Number.POSITIVE_INFINITY;
      for (let i = 0; i < path.length; i += 1) {
        const point = path[i];
        const dist = Math.hypot(point.x - target.x, point.y - target.y, point.z - target.z);
        if (dist < bestDist) {
          bestDist = dist;
          bestIndex = i;
        }
      }
      return bestIndex;
    }

    function pathLength(path) {
      if (!path || path.length < 2) return 0;
      let total = 0;
      for (let i = 1; i < path.length; i += 1) {
        const a = path[i - 1];
        const b = path[i];
        total += Math.hypot(b.x - a.x, b.y - a.y, b.z - a.z);
      }
      return total;
    }

    function buildUrbanTowers(path, options = {}) {
      if (!path || path.length < 3) return [];
      const towers = [];
      const laneGap = options.laneGap || 210;
      const outerGap = options.outerGap || 360;
      const depth = options.depth || 80;
      const minHeight = options.minHeight || 300;
      const maxHeight = options.maxHeight || 760;
      const step = options.step || 3;
      const skipStart = options.skipStart || 2;
      const skipEnd = options.skipEnd || 2;
      for (let i = skipStart; i < path.length - skipEnd; i += step) {
        const node = path[i];
        const next = path[Math.min(path.length - 1, i + 1)];
        const dx = next.x - node.x;
        const dz = next.z - node.z;
        const len = Math.max(1, Math.hypot(dx, dz));
        const nx = dz / len;
        const nz = -dx / len;
        const towerCount = 2 + (i % 2);
        for (const side of [-1, 1]) {
          for (let j = 0; j < towerCount; j += 1) {
            const radial = laneGap + outerGap * (j + 1) + (i % 3) * 34 + j * 16;
            const along = (j - (towerCount - 1) * 0.5) * 170 + ((i + j) % 2 === 0 ? 60 : -60);
            const x = node.x + nx * side * radial + (dx / len) * along;
            const z = node.z + nz * side * radial + (dz / len) * along;
            const h = minHeight + ((i * 97 + j * 71 + (side > 0 ? 37 : 0)) % (maxHeight - minHeight));
            towers.push({
              type: "tower",
              x,
              y: groundY,
              z,
              h,
              w: 90 + ((i + j) % 3) * 24,
              d: depth + (j % 2) * 18,
              glow: 0.16 + ((i + j) % 4) * 0.03,
              hit: false,
            });
          }
        }
      }
      return towers;
    }

    function makeCourse(round, courseSpec = null) {
      const diff = roundDifficulty(round);
      const gates = [];
      const clouds = [];
      const coursePath = (courseSpec?.entryPath || [
        { x: 82, y: -95, z: -220, heading: 0 },
        { x: 82, y: -70, z: 260, heading: 0 },
        { x: 42, y: 35, z: 920, heading: 0 },
      ]).map((node) => ({ ...node }));
      const layoutScaleX = courseSpec?.layoutScaleX ?? 1.18;
      const layoutScaleZ = courseSpec?.layoutScaleZ ?? 1.24;
      const layoutScaleY = courseSpec?.layoutScaleY ?? 1.16;
      const fixedLayouts = {
        1: [
          { x: 0, z: 1700, y: 58, h: 0.00 },
          { x: -650, z: 3400, y: 62 },
          { x: -1450, z: 4800, y: 58 },
          { x: -2500, z: 5200, y: 64 },
          { x: -3300, z: 3900, y: 60 },
          { x: -3600, z: 2100, y: 64 },
          { x: -3200, z: 300, y: 58 },
          { x: -2500, z: -1300, y: 62 },
          { x: -1400, z: -2100, y: 58 },
          { x: -450, z: -1700, y: 60 },
          { x: 0, z: -900, y: 58, h: 0.00 },
        ],
        2: [
          { x: 0, z: 1850, y: 54, h: 0.00 },
          { x: -520, z: 3650, y: 96 },
          { x: -1350, z: 5350, y: 28 },
          { x: -2450, z: 6550, y: 112 },
          { x: -3650, z: 6500, y: 48 },
          { x: -4550, z: 5200, y: 98 },
          { x: -4850, z: 3300, y: 42 },
          { x: -4450, z: 1250, y: 106 },
          { x: -3450, z: -600, y: 54 },
          { x: -2200, z: -2100, y: 92 },
          { x: -900, z: -2400, y: 46 },
          { x: -180, z: -1500, y: 72 },
          { x: 0, z: -900, y: 58, h: 0.00 },
        ],
        3: [
          { x: 0, z: 1950, y: 44, h: 0.00 },
          { x: -480, z: 3950, y: 170 },
          { x: -1300, z: 5900, y: -10 },
          { x: -2550, z: 7550, y: 230 },
          { x: -4050, z: 7950, y: 24 },
          { x: -5450, z: 6900, y: 285 },
          { x: -6100, z: 5000, y: -20 },
          { x: -5900, z: 2750, y: 245 },
          { x: -4950, z: 600, y: 32 },
          { x: -3450, z: -1300, y: 210 },
          { x: -1850, z: -2750, y: 0 },
          { x: -650, z: -2300, y: 150 },
          { x: 0, z: -950, y: 60, h: 0.00 },
        ],
        4: [
          { x: 0, z: 2100, y: 20, h: 0.00 },
          { x: -850, z: 4300, y: 240 },
          { x: -2400, z: 6400, y: -35 },
          { x: -4700, z: 7900, y: 360 },
          { x: -7300, z: 7600, y: 40 },
          { x: -9200, z: 5600, y: 420 },
          { x: -9000, z: 2900, y: -45 },
          { x: -6900, z: 900, y: 320 },
          { x: -3900, z: 350, y: 10 },
          { x: -1200, z: 1400, y: 380 },
          { x: 1400, z: 3600, y: -20 },
          { x: 3100, z: 2500, y: 300 },
          { x: 2600, z: 100, y: 40 },
          { x: 1200, z: -1800, y: 260 },
          { x: 0, z: -1050, y: 60, h: 0.00 },
        ],
        5: [
          { x: 0, z: 2200, y: -20, h: 0.00 },
          { x: -900, z: 4400, y: 300 },
          { x: -2600, z: 6600, y: -60 },
          { x: -5300, z: 8400, y: 460 },
          { x: -8600, z: 8200, y: -35 },
          { x: -11100, z: 6100, y: 500 },
          { x: -11200, z: 3100, y: -70 },
          { x: -9200, z: 900, y: 420 },
          { x: -6200, z: 200, y: -25 },
          { x: -3300, z: 1600, y: 360 },
          { x: -1200, z: 4300, y: -55 },
          { x: -2600, z: 6200, y: 480 },
          { x: -5400, z: 5600, y: 20 },
          { x: -6400, z: 3300, y: 440 },
          { x: -4300, z: 800, y: -65 },
          { x: -1000, z: -900, y: 390 },
          { x: 2600, z: 200, y: -40 },
          { x: 3800, z: 3000, y: 430 },
          { x: 1600, z: 1600, y: 30 },
          { x: 500, z: -2100, y: 320 },
          { x: 0, z: -1100, y: 60, h: 0.00 },
        ],
      };
      let fixedLayout = courseSpec?.fixedLayout || fixedLayouts[round] || null;
      if (!courseSpec && fixedLayout && round === 1) fixedLayout = fixedLayout.slice(0, 7);
      if (!courseSpec && fixedLayout && round === 2) fixedLayout = fixedLayout.slice(0, 9);
      let x = 0;
      let z = 1280;
      let heading = 0;
      const gateCount = fixedLayout ? fixedLayout.length : 12;
      for (let i = 0; i < gateCount; i += 1) {
        const prev = gates[gates.length - 1] || { x: 0, y: 35, z: 0 };
        let y = 35;
        if (fixedLayout) {
          const node = fixedLayout[i];
          const prevNode = fixedLayout[Math.max(0, i - 1)];
          const nextNode = fixedLayout[Math.min(fixedLayout.length - 1, i + 1)];
          x = node.x * layoutScaleX;
          z = node.z * layoutScaleZ;
          y = node.y * layoutScaleY;
          heading = node.h ?? Math.atan2(
            (nextNode.x - prevNode.x) * layoutScaleX,
            (nextNode.z - prevNode.z) * layoutScaleZ
          );
        } else {
          heading += (Math.sin(i * 1.27) * 0.46 + (i % 4 === 1 ? 0.34 : i % 4 === 3 ? -0.40 : 0)) * diff.turnScale;
          const spacing = (980 + (i % 3) * 180) * diff.spacingScale;
          x += Math.sin(heading) * spacing * diff.lateralScale;
          z += Math.cos(heading) * spacing;
          y = 35 + Math.sin(i * 0.62) * 170 * diff.verticalScale + (round >= 4 ? Math.sin(i * 1.8) * 120 : 0);
        }
        gates.push({ x, y, z, heading, passed: false });
        coursePath.push({ x, y, z, heading });
      }
      const smoothCoursePath = buildSmoothCoursePath(coursePath, courseSpec?.pathSubdivisions || 14);
      const courseMarkers = sampleGuideMarkers(smoothCoursePath, courseSpec?.markerSpacing || (round <= 2 ? 420 : 360));
      const gatePathIndices = gates.map((gate) => nearestPathIndex(smoothCoursePath, gate));

      const pylons = [];
      const scenery = [];
      for (let i = 0; i < gates.length; i += 1) {
        const gate = gates[i];
        const nx = Math.cos(gate.heading);
        const nz = -Math.sin(gate.heading);
        const gatePylonOffset = gateWorldRadius + 36;
        const topHeight = Math.max(160, gate.y - groundY + gateWorldRadius + 12);
        pylons.push({
          x: gate.x + nx * gatePylonOffset,
          y: groundY,
          z: gate.z + nz * gatePylonOffset,
          h: topHeight,
          gateEdge: true,
          hit: false,
        });
        pylons.push({
          x: gate.x - nx * gatePylonOffset,
          y: groundY,
          z: gate.z - nz * gatePylonOffset,
          h: topHeight,
          gateEdge: true,
          hit: false,
        });
        if (i % 2 === 0) {
          const side = i % 4 === 0 ? 1 : -1;
          clouds.push({
            x: gate.x + nx * side * (420 + i * 22),
            y: 650 + (i % 3) * 42,
            z: gate.z + nz * side * 180 + 420,
            size: 180 + (i % 4) * 36,
            alpha: 0.20 + (i % 3) * 0.04,
          });
        }
      }
      if (round === 9) {
        scenery.push(...buildUrbanTowers(smoothCoursePath, {
          laneGap: 300,
          outerGap: 390,
          depth: 92,
          minHeight: 320,
          maxHeight: 860,
          step: 4,
          skipStart: 2,
          skipEnd: 1,
        }));
      }
      const boundsX = [0, 82, ...gates.map((gate) => gate.x)];
      const boundsZ = [-520, 0, ...smoothCoursePath.map((point) => point.z)];
      const courseBounds = {
        minX: Math.floor((Math.min(...boundsX) - 1200) / 200) * 200,
        maxX: Math.ceil((Math.max(...boundsX) + 1200) / 200) * 200,
        minZ: Math.floor((Math.min(...boundsZ) - 1200) / 200) * 200,
        maxZ: Math.ceil((Math.max(...boundsZ) + 1800) / 200) * 200,
      };
      return { gates, pylons, scenery, clouds, courseMarkers, gatePathIndices, coursePath: smoothCoursePath, courseBounds, courseSpec };
    }

    function makeCpuRival(round, coursePath, laneOffset, label, paceMul, profile = null) {
      const rivalAircraft = profile?.aircraft || (round >= 4 ? AIRCRAFTS[2] : round === 3 ? AIRCRAFTS[1] : AIRCRAFTS[0]);
      const courseDistance = Math.max(9600, pathLength(coursePath));
      const baseCruise = 430 + round * 34;
      const raceTime = courseDistance / baseCruise;
      const landingDuration = 4.8;
      return {
        active: true,
        kind: "cpu",
        aircraft: rivalAircraft,
        name: label,
        profileId: profile?.id || label,
        style: profile?.style || "steady",
        roleLabel: profile?.label || (round === 2 ? "ROOKIE" : "CPU"),
        lineWave: profile?.wave || 0,
        seed: (profile?.id || label).length * 0.83,
        raceTime: raceTime * paceMul,
        landingDuration,
        totalTime: raceTime * paceMul + landingDuration,
        laneOffset,
        progress: 0,
        x: 82 + laneOffset,
        y: -95,
        z: -220,
        heading: 0,
        finished: false,
      };
    }

    function makeGhostRival(saved, laneOffset, label, profileId = null) {
      const ghostAircraft = AIRCRAFTS[saved?.samples?.length ? saved.samples.length % AIRCRAFTS.length : 0] || AIRCRAFTS[0];
      const first = saved.samples[0] || { x: 82, y: -95, z: -220, h: 0, r: 0, s: 0 };
      return {
        active: true,
        kind: "ghost",
        aircraft: ghostAircraft,
        name: label || saved.name || "GHOST",
        status: saved.status || "finish",
        profileId: profileId || saved.name || "ghost",
        style: "ghost",
        roleLabel: "GHOST",
        lineWave: 0,
        seed: (saved.name || "ghost").length * 0.91,
        totalTime: saved.time_ms / 1000,
        samples: saved.samples,
        laneOffset,
        progress: 0,
        x: first.x + laneOffset,
        y: first.y,
        z: first.z,
        heading: first.h,
        roll: first.r || 0,
        finished: false,
      };
    }

    function uniqueGhostCandidates() {
      if (!USE_GHOST_RIVALS) return [];
      const playerName = (localStorage.getItem("airrace_name") || "").trim().toLowerCase();
      const usedNames = new Set();
      if (playerName) usedNames.add(playerName);
      const ghosts = (game?.roundGhosts || []).filter((g) => Array.isArray(g.samples) && g.samples.length >= 2);
      const selected = [];
      for (const ghost of ghosts) {
        const rawName = (ghost?.name || "ghost").trim();
        const key = rawName.toLowerCase();
        if (usedNames.has(key)) continue;
        usedNames.add(key);
        selected.push(ghost);
      }
      return selected;
    }

    function makeRivals(round, coursePath) {
      if (round === 1) return [];
      const mode = selectedFieldMode();
      const savedGhosts = uniqueGhostCandidates();
      if (round >= GRAND_PRIX_START_ROUND) {
        const tournament = game?.tournament || createTournamentState();
        return tournament.rivals.map((profile, index) => {
          const saved = savedGhosts[index];
          if (saved) {
            return makeGhostRival(saved, profile.laneOffset, saved.name || profile.name, profile.id);
          }
          const paceMul = profile.paceByRound?.[round] || 1.0;
          return makeCpuRival(round, coursePath, profile.laneOffset, profile.name, paceMul, profile);
        });
      }
      const rivals = [];
      const practiceCount = Math.max(0, mode.round2Size - 1);
      const practiceOffsets = buildLaneOffsets(practiceCount);
      for (let i = 0; i < practiceCount; i += 1) {
        if (savedGhosts[i]) {
          rivals.push(makeGhostRival(savedGhosts[i], practiceOffsets[i], savedGhosts[i].name || `GHOST ${i + 1}`));
          continue;
        }
        const rookieType = i % 2 === 0 ? aircraftById("skylancer") : aircraftById("thunderbolt");
        const rookiePace = 1.18 + i * 0.05;
        rivals.push(makeCpuRival(round, coursePath, practiceOffsets[i], `ROOKIE ${String.fromCharCode(65 + i)}`, rookiePace, {
          id: `rookie_${i}`,
          style: i % 3 === 1 ? "line" : "steady",
          label: "ROOKIE",
          aircraft: rookieType,
          wave: 3 + i,
        }));
      }
      return rivals;
    }

    function saveGhost(record) {
      if (!record.samples || record.samples.length < 20) return;
      fetch("/api/airrace/ghosts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(record),
      })
        .then((res) => {
          if (res.ok) delete window.airraceRoundCache[record.round];
        })
        .catch(() => {});
    }
