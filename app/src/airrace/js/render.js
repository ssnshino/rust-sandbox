    function roundTheme(round) {
      const themes = {
        1: { skyTop: "#153355", skyMid: "#2c6d96", skyLow: "#9ed8ff", skyBottom: "#ebf8ff", groundTop: "#58703a", groundMid: "#33461f", groundBottom: "#12180d" },
        2: { skyTop: "#17314d", skyMid: "#4374a1", skyLow: "#b7e3ff", skyBottom: "#f2fbff", groundTop: "#7a6338", groundMid: "#514027", groundBottom: "#1b150d" },
        3: { skyTop: "#3a1f1a", skyMid: "#b35b3c", skyLow: "#f3bf73", skyBottom: "#fff1da", groundTop: "#8a5632", groundMid: "#5b351f", groundBottom: "#24150d" },
        4: { skyTop: "#26456a", skyMid: "#6aa0cf", skyLow: "#d8f0ff", skyBottom: "#f7fcff", groundTop: "#6d7f4d", groundMid: "#425034", groundBottom: "#172016" },
        5: { skyTop: "#1f4e72", skyMid: "#3ca0d0", skyLow: "#ffe0a8", skyBottom: "#fff6e5", groundTop: "#5a6b36", groundMid: "#344222", groundBottom: "#14190f" },
        6: { skyTop: "#3d556c", skyMid: "#89abc6", skyLow: "#e4f3ff", skyBottom: "#fbfdff", groundTop: "#4d6440", groundMid: "#31422b", groundBottom: "#121910" },
        7: { skyTop: "#30404f", skyMid: "#6f879b", skyLow: "#d6e3ef", skyBottom: "#f7fafc", groundTop: "#655942", groundMid: "#403729", groundBottom: "#17130f" },
        8: { skyTop: "#183a63", skyMid: "#4f8ec3", skyLow: "#d9ecff", skyBottom: "#fbfdff", groundTop: "#59636c", groundMid: "#363d44", groundBottom: "#13171b" },
        9: { skyTop: "#6a3018", skyMid: "#d17835", skyLow: "#ffd68d", skyBottom: "#fff3db", groundTop: "#956639", groundMid: "#5e3f22", groundBottom: "#24170d" },
        10: { skyTop: "#10264e", skyMid: "#305f9a", skyLow: "#bfe0ff", skyBottom: "#f4fbff", groundTop: "#506746", groundMid: "#32412d", groundBottom: "#111710" },
      };
      return themes[round] || themes[3];
    }

    function drawBackdropLandmarks(round, leftHorizon, rightHorizon) {
      const baseY = (leftHorizon.y + rightHorizon.y) * 0.5;
      const drawBand = (color, points) => {
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.moveTo(points[0][0], points[0][1]);
        for (let i = 1; i < points.length; i += 1) ctx.lineTo(points[i][0], points[i][1]);
        ctx.lineTo(W, H);
        ctx.lineTo(0, H);
        ctx.closePath();
        ctx.fill();
      };
      const drawSkyline = (color, startX, width, heights) => {
        ctx.fillStyle = color;
        let x = startX;
        for (let i = 0; i < heights.length; i += 1) {
          const w = width * (0.72 + (i % 3) * 0.2);
          const h = heights[i];
          ctx.fillRect(x, baseY - h, w, h);
          x += w + 4;
        }
      };
      const drawRiver = (colorA, colorB) => {
        ctx.strokeStyle = colorA;
        ctx.lineWidth = 16;
        ctx.beginPath();
        ctx.moveTo(W * 0.10, baseY + 34);
        ctx.bezierCurveTo(W * 0.24, baseY + 8, W * 0.38, baseY + 66, W * 0.50, baseY + 18);
        ctx.bezierCurveTo(W * 0.64, baseY - 18, W * 0.82, baseY + 34, W * 0.94, baseY + 6);
        ctx.stroke();
        ctx.strokeStyle = colorB;
        ctx.lineWidth = 6;
        ctx.stroke();
      };
      const drawWindmills = () => {
        ctx.strokeStyle = "rgba(245,248,252,0.34)";
        ctx.lineWidth = 2;
        [0.16, 0.31, 0.74, 0.86].forEach((p, i) => {
          const x = W * p;
          const y = baseY + 18 + (i % 2) * 10;
          ctx.beginPath();
          ctx.moveTo(x, y);
          ctx.lineTo(x, y - 46);
          ctx.stroke();
          ctx.beginPath();
          ctx.moveTo(x, y - 46);
          ctx.lineTo(x - 16, y - 56);
          ctx.moveTo(x, y - 46);
          ctx.lineTo(x + 18, y - 42);
          ctx.moveTo(x, y - 46);
          ctx.lineTo(x - 4, y - 26);
          ctx.stroke();
        });
      };
      const drawTowers = () => {
        ctx.fillStyle = "rgba(185, 202, 215, 0.26)";
        ctx.fillRect(W * 0.18, baseY - 68, 16, 68);
        ctx.fillRect(W * 0.72, baseY - 76, 20, 76);
        ctx.fillRect(W * 0.78, baseY - 56, 12, 56);
      };
      const drawHarborCranes = () => {
        ctx.strokeStyle = "rgba(214, 226, 236, 0.28)";
        ctx.lineWidth = 3;
        [0.20, 0.32, 0.68, 0.82].forEach((p) => {
          const x = W * p;
          ctx.beginPath();
          ctx.moveTo(x, baseY + 14);
          ctx.lineTo(x, baseY - 36);
          ctx.lineTo(x + 26, baseY - 54);
          ctx.stroke();
        });
      };
      const drawBridgeBands = (color, yOffset) => {
        ctx.strokeStyle = color;
        ctx.lineWidth = 3;
        [0.12, 0.46].forEach((start, idx) => {
          ctx.beginPath();
          ctx.moveTo(W * start, baseY + yOffset + idx * 10);
          ctx.bezierCurveTo(W * (start + 0.10), baseY + yOffset - 10, W * (start + 0.26), baseY + yOffset - 12, W * (start + 0.38), baseY + yOffset + 4);
          ctx.stroke();
        });
      };
      const drawNeedleTower = (xRatio, height, color) => {
        const x = W * xRatio;
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(x, baseY);
        ctx.lineTo(x, baseY - height);
        ctx.lineTo(x + 10, baseY - height + 14);
        ctx.stroke();
      };
      const drawMountainBand = (color) => {
        drawBand(color, [
          [0, baseY + 18],
          [W * 0.10, baseY - 12],
          [W * 0.20, baseY + 8],
          [W * 0.34, baseY - 34],
          [W * 0.46, baseY + 2],
          [W * 0.60, baseY - 46],
          [W * 0.76, baseY + 10],
          [W * 0.88, baseY - 18],
          [W, baseY + 6],
        ]);
      };

      if (round === 3 || round === 9) {
        drawBand("rgba(121, 74, 36, 0.34)", [[0, baseY + 10], [W * 0.18, baseY - 12], [W * 0.36, baseY + 8], [W * 0.58, baseY - 18], [W * 0.82, baseY + 4], [W, baseY - 10]]);
      } else if (round === 4) {
        drawRiver("rgba(123, 196, 240, 0.24)", "rgba(237, 248, 255, 0.18)");
        drawSkyline("rgba(52, 74, 96, 0.24)", W * 0.08, 18, [36, 42, 58, 34, 50, 70, 30]);
        drawSkyline("rgba(52, 74, 96, 0.20)", W * 0.62, 18, [28, 36, 54, 44, 38, 60]);
      } else if (round === 5) {
        ctx.fillStyle = "rgba(108, 195, 224, 0.18)";
        ctx.fillRect(0, baseY + 22, W, 24);
        drawBand("rgba(75, 103, 82, 0.24)", [[0, baseY + 12], [W * 0.18, baseY - 8], [W * 0.40, baseY + 14], [W * 0.72, baseY - 6], [W, baseY + 8]]);
      } else if (round === 6) {
        ctx.fillStyle = "rgba(132, 185, 214, 0.16)";
        ctx.fillRect(0, baseY + 20, W, 22);
        drawHarborCranes();
        drawWindmills();
      } else if (round === 7) {
        drawSkyline("rgba(70, 72, 76, 0.24)", W * 0.10, 20, [26, 48, 34, 62, 30, 40, 56, 36]);
        drawTowers();
      } else if (round === 8) {
        drawBridgeBands("rgba(156, 188, 228, 0.28)", 24);
        drawSkyline("rgba(63, 83, 112, 0.30)", W * 0.05, 16, [44, 72, 58, 92, 64, 84, 50, 68, 96]);
        drawSkyline("rgba(63, 83, 112, 0.22)", W * 0.56, 14, [50, 82, 60, 74, 88, 54, 70, 92]);
        drawNeedleTower(0.18, 92, "rgba(210, 228, 250, 0.24)");
        drawNeedleTower(0.82, 106, "rgba(210, 228, 250, 0.24)");
      } else if (round === 9) {
        drawSkyline("rgba(96, 78, 56, 0.26)", W * 0.04, 16, [28, 42, 58, 92, 66, 120, 74, 98, 52]);
        drawSkyline("rgba(96, 78, 56, 0.18)", W * 0.58, 15, [40, 62, 96, 70, 110, 56, 84]);
        drawNeedleTower(0.26, 120, "rgba(245, 222, 178, 0.22)");
        drawNeedleTower(0.74, 132, "rgba(245, 222, 178, 0.22)");
      } else if (round === 10) {
        drawRiver("rgba(95, 163, 231, 0.18)", "rgba(227, 242, 255, 0.10)");
        drawMountainBand("rgba(76, 98, 126, 0.20)");
        drawSkyline("rgba(57, 78, 112, 0.24)", W * 0.08, 16, [38, 56, 48, 78, 52, 70, 44]);
        drawSkyline("rgba(57, 78, 112, 0.18)", W * 0.62, 15, [42, 70, 54, 92, 58, 74]);
        drawNeedleTower(0.68, 84, "rgba(220, 236, 255, 0.22)");
        drawBand("rgba(70, 92, 108, 0.18)", [[0, baseY + 14], [W * 0.22, baseY - 18], [W * 0.48, baseY + 10], [W * 0.68, baseY - 28], [W, baseY + 8]]);
      }
    }

    function projectPoint(x, y, z) {
      const dx = x - game.planeX;
      const dz = z - game.planeZ;
      const cos = Math.cos(game.heading);
      const sin = Math.sin(game.heading);
      const camX = dx * cos - dz * sin;
      const relZ = dx * sin + dz * cos;
      if (relZ <= 40) return null;
      const scale = 270 / relZ;
      const rawX = centerX + camX * scale;
      const rawY = horizon - (y - playerWorldY()) * scale;
      const rotated = rollPoint(rawX, rawY);
      return {
        x: rotated.x,
        y: rotated.y,
        s: scale,
        z: relZ,
      };
    }

    function interpolatePath(path, t) {
      if (!path || path.length === 0) return null;
      const scaled = Math.max(0, Math.min(path.length - 1.001, t * (path.length - 1)));
      const i = Math.floor(scaled);
      const f = scaled - i;
      const a = path[i];
      const b = path[Math.min(path.length - 1, i + 1)];
      return {
        x: a.x + (b.x - a.x) * f,
        y: a.y + (b.y - a.y) * f,
        z: a.z + (b.z - a.z) * f,
        heading: a.heading + (b.heading - a.heading) * f,
      };
    }

    function rollPoint(x, y) {
      const angle = -game.roll * rollVisual;
      const dx = x - centerX;
      const dy = y - centerY;
      const cos = Math.cos(angle);
      const sin = Math.sin(angle);
      return {
        x: centerX + dx * cos - dy * sin,
        y: centerY + dx * sin + dy * cos,
      };
    }

    function drawLine(a, b, color, width = 1) {
      if (!a || !b) return;
      ctx.strokeStyle = color;
      ctx.lineWidth = width;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }

    function fillPoly(a, b, c, d, color) {
      if (!a || !b || !c || !d) return;
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.lineTo(c.x, c.y);
      ctx.lineTo(d.x, d.y);
      ctx.closePath();
      ctx.fill();
    }

    function fillGroundQuad(points, color) {
      const projected = points.map((p) => projectPoint(p.x, p.y, p.z));
      if (projected.some((p) => !p)) return;
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.moveTo(projected[0].x, projected[0].y);
      for (let i = 1; i < projected.length; i += 1) ctx.lineTo(projected[i].x, projected[i].y);
      ctx.closePath();
      ctx.fill();
    }

    function drawRolledBackground(leftHorizon, rightHorizon) {
      const theme = roundTheme(game?.round || 1);
      const sky = ctx.createLinearGradient(0, 0, 0, H);
      sky.addColorStop(0, theme.skyTop);
      sky.addColorStop(0.35, theme.skyMid);
      sky.addColorStop(0.62, theme.skyLow);
      sky.addColorStop(1, theme.skyBottom);
      ctx.fillStyle = sky;
      ctx.fillRect(0, 0, W, H);

      const sunGlow = ctx.createRadialGradient(centerX, Math.max(48, horizon - H * 0.2), 12, centerX, Math.max(48, horizon - H * 0.2), W * 0.45);
      sunGlow.addColorStop(0, "rgba(255,255,255,.18)");
      sunGlow.addColorStop(0.25, "rgba(125,211,252,.14)");
      sunGlow.addColorStop(1, "rgba(2,6,23,0)");
      ctx.fillStyle = sunGlow;
      ctx.fillRect(0, 0, W, H);

      drawBackdropLandmarks(game?.round || 1, leftHorizon, rightHorizon);

      const ground = ctx.createLinearGradient(0, horizon, 0, H);
      ground.addColorStop(0, theme.groundTop);
      ground.addColorStop(0.4, theme.groundMid);
      ground.addColorStop(1, theme.groundBottom);
      ctx.fillStyle = ground;
      ctx.beginPath();
      ctx.moveTo(leftHorizon.x, leftHorizon.y);
      ctx.lineTo(rightHorizon.x, rightHorizon.y);
      ctx.lineTo(W, H);
      ctx.lineTo(0, H);
      ctx.closePath();
      ctx.fill();
    }

    function drawGrid() {
      const leftHorizon = rollPoint(0, horizon);
      const rightHorizon = rollPoint(W, horizon);
      drawRolledBackground(leftHorizon, rightHorizon);

      drawClouds();
      drawGroundDecor();

      ctx.strokeStyle = "rgba(125,249,255,.35)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(leftHorizon.x, leftHorizon.y);
      ctx.lineTo(rightHorizon.x, rightHorizon.y);
      ctx.stroke();

      drawWorldGroundGrid();
    }

    function drawGroundDecor() {
      const bounds = game.courseBounds;
      if (!bounds) return;
      const y = groundY + 1;
      const minX = bounds.minX;
      const maxX = bounds.maxX;
      const minZ = bounds.minZ;
      const maxZ = bounds.maxZ;
      const round = game.round || 1;

      if (round === 4 || round === 10) {
        fillGroundQuad([
          { x: minX + 300, y, z: minZ + 900 },
          { x: minX + 900, y, z: maxZ - 400 },
          { x: minX + 1500, y, z: maxZ - 600 },
          { x: minX + 900, y, z: minZ + 700 },
        ], round === 10 ? "rgba(86, 156, 220, 0.22)" : "rgba(112, 186, 228, 0.24)");
      }

      if (round === 5) {
        fillGroundQuad([
          { x: maxX - 1400, y, z: minZ + 600 },
          { x: maxX + 100, y, z: minZ + 900 },
          { x: maxX + 100, y, z: maxZ - 200 },
          { x: maxX - 1700, y, z: maxZ - 500 },
        ], "rgba(76, 172, 216, 0.20)");
      }

      if (round === 6) {
        for (let i = 0; i < 5; i += 1) {
          const x0 = minX + 500 + i * 900;
          fillGroundQuad([
            { x: x0, y, z: minZ + 700 },
            { x: x0 + 260, y, z: minZ + 700 },
            { x: x0 + 340, y, z: maxZ - 500 },
            { x: x0 + 80, y, z: maxZ - 500 },
          ], "rgba(124, 186, 216, 0.16)");
        }
      }

      if (round === 7) {
        for (let i = 0; i < 6; i += 1) {
          const x0 = minX + 700 + i * 1200;
          fillGroundQuad([
            { x: x0, y, z: minZ + 1400 },
            { x: x0 + 520, y, z: minZ + 1400 },
            { x: x0 + 620, y, z: maxZ - 900 },
            { x: x0 + 100, y, z: maxZ - 900 },
          ], "rgba(84, 86, 92, 0.14)");
        }
      }

      if (round === 8 || round === 10) {
        for (let i = 0; i < 7; i += 1) {
          const x0 = minX + 600 + i * 950;
          fillGroundQuad([
            { x: x0, y, z: minZ + 1000 },
            { x: x0 + 140, y, z: minZ + 1000 },
            { x: x0 + 220, y, z: maxZ - 700 },
            { x: x0 + 80, y, z: maxZ - 700 },
          ], "rgba(170, 190, 210, 0.08)");
        }
      }

      if (round === 8) {
        for (let i = 0; i < 5; i += 1) {
          const z0 = minZ + 1400 + i * 1800;
          fillGroundQuad([
            { x: minX + 900, y, z: z0 },
            { x: maxX - 700, y, z: z0 + 180 },
            { x: maxX - 820, y, z: z0 + 420 },
            { x: minX + 720, y, z: z0 + 240 },
          ], "rgba(110, 132, 156, 0.12)");
        }
      }

      if (round === 9) {
        for (let i = 0; i < 6; i += 1) {
          fillGroundQuad([
            { x: minX + i * 1800, y, z: minZ + 1800 + i * 400 },
            { x: minX + 900 + i * 1800, y, z: minZ + 1200 + i * 420 },
            { x: minX + 1600 + i * 1800, y, z: minZ + 2400 + i * 440 },
            { x: minX + 700 + i * 1800, y, z: minZ + 3000 + i * 420 },
          ], "rgba(219, 167, 97, 0.10)");
        }
        for (let i = 0; i < 6; i += 1) {
          const x0 = minX + 900 + i * 1500;
          fillGroundQuad([
            { x: x0, y, z: minZ + 900 },
            { x: x0 + 220, y, z: minZ + 900 },
            { x: x0 + 360, y, z: maxZ - 600 },
            { x: x0 + 140, y, z: maxZ - 600 },
          ], "rgba(110, 86, 58, 0.12)");
        }
      }

      if (round === 10) {
        fillGroundQuad([
          { x: minX + 1200, y, z: minZ + 600 },
          { x: minX + 1800, y, z: maxZ - 500 },
          { x: minX + 2500, y, z: maxZ - 700 },
          { x: minX + 1900, y, z: minZ + 400 },
        ], "rgba(82, 150, 220, 0.20)");
        for (let i = 0; i < 5; i += 1) {
          const z0 = minZ + 1200 + i * 1700;
          fillGroundQuad([
            { x: minX + 2800, y, z: z0 },
            { x: maxX - 900, y, z: z0 + 120 },
            { x: maxX - 1040, y, z: z0 + 300 },
            { x: minX + 2660, y, z: z0 + 180 },
          ], "rgba(170, 190, 210, 0.10)");
        }
      }

      if (round === 1 || round === 2) {
        for (let i = 0; i < 6; i += 1) {
          fillGroundQuad([
            { x: minX + 400 + i * 1000, y, z: minZ + 1000 },
            { x: minX + 1050 + i * 1000, y, z: minZ + 1000 },
            { x: minX + 980 + i * 1000, y, z: maxZ - 600 },
            { x: minX + 320 + i * 1000, y, z: maxZ - 600 },
          ], i % 2 === 0 ? "rgba(114, 160, 88, 0.10)" : "rgba(170, 138, 82, 0.08)");
        }
      }
    }

    function drawWorldGroundGrid() {
      const step = 200;
      const bounds = game.courseBounds || {
        minX: -2200,
        maxX: 2200,
        minZ: -1200,
        maxZ: 5200,
      };
      const minX = Math.min(bounds.minX, Math.floor((game.planeX - 900) / step) * step);
      const maxX = Math.max(bounds.maxX, Math.ceil((game.planeX + 900) / step) * step);
      const minZ = Math.min(bounds.minZ, Math.floor((game.planeZ - 600) / step) * step);
      const maxZ = Math.max(bounds.maxZ, Math.ceil((game.planeZ + 2400) / step) * step);

      for (let x = minX; x <= maxX; x += step) {
        drawLine(
          projectPoint(x, groundY, minZ),
          projectPoint(x, groundY, maxZ),
          x === 0 ? "rgba(250,204,21,.30)" : "rgba(250,204,21,.18)"
        );
      }

      for (let z = minZ; z <= maxZ; z += step) {
        drawLine(
          projectPoint(minX, groundY, z),
          projectPoint(maxX, groundY, z),
          z === 0 ? "rgba(125,249,255,.24)" : "rgba(251,191,36,.15)"
        );
      }
    }

    function drawClouds() {
      if (!game.clouds || game.clouds.length === 0) return;
      for (const cloud of game.clouds) {
        const p = projectPoint(cloud.x, cloud.y, cloud.z);
        if (!p || p.z > 3600) continue;
        const visible = Math.max(0, Math.min(0.82, (game.altitude - 700) / 180));
        if (visible <= 0.02) continue;
        const radius = Math.max(16, cloud.size * p.s);
        ctx.fillStyle = `rgba(236, 253, 255, ${(cloud.alpha || 0.22) * visible})`;
        ctx.beginPath();
        ctx.ellipse(p.x, p.y, radius, radius * 0.42, -game.roll * rollVisual, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = `rgba(125, 211, 252, ${(cloud.alpha || 0.22) * 0.42 * visible})`;
        ctx.beginPath();
        ctx.ellipse(p.x + radius * 0.2, p.y + radius * 0.08, radius * 0.72, radius * 0.28, -game.roll * rollVisual, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    function groundPoint(side, forward) {
      const sin = Math.sin(game.heading);
      const cos = Math.cos(game.heading);
      const x = game.planeX + sin * forward + cos * side;
      const z = game.planeZ + cos * forward - sin * side;
      return projectPoint(x, groundY, z);
    }

    function drawGate(gate, index) {
      const p = projectPoint(gate.x, gate.y, gate.z);
      if (!p || p.z > 3200) return;

      const r = gateWorldRadius * p.s;
      const color = gate.passed
        ? "#facc15"
        : index === game.gateIndex
          ? "#67e8f9"
          : "rgba(148,163,184,.35)";
      const glow = gate.passed
        ? "rgba(250,204,21,.34)"
        : index === game.gateIndex
          ? "rgba(34,211,238,.45)"
          : "rgba(148,163,184,.14)";

      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(p.x, p.y, r * 1.05, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = color;
      ctx.lineWidth = Math.max(1, 3 * p.s);
      ctx.beginPath();
      ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      ctx.stroke();
      ctx.strokeStyle = "rgba(255,255,255,.35)";
      ctx.lineWidth = Math.max(1, 1.2 * p.s);
      ctx.beginPath();
      ctx.arc(p.x, p.y, r * 0.68, 0, Math.PI * 2);
      ctx.stroke();

      ctx.fillStyle = color;
      ctx.font = `${Math.max(10, 22 * p.s)}px DotGothic16`;
      ctx.textAlign = "center";
      ctx.fillText(`${index + 1}`, p.x, p.y - r - 6);
    }

    function drawPylon(pylon) {
      const base = projectPoint(pylon.x, pylon.y ?? groundY, pylon.z);
      const top = projectPoint(pylon.x, (pylon.y ?? groundY) + (pylon.h || 120), pylon.z);
      if (!base || !top || base.z > 2800) return;
      const w = Math.max(3, 15 * base.s);

      ctx.strokeStyle = pylon.hit ? "rgba(239,68,68,.35)" : "#fb923c";
      ctx.lineWidth = w;
      ctx.beginPath();
      ctx.moveTo(base.x, base.y);
      ctx.lineTo(top.x, top.y);
      ctx.stroke();

      ctx.strokeStyle = "#fed7aa";
      ctx.lineWidth = Math.max(1, w * 0.28);
      ctx.beginPath();
      ctx.moveTo(base.x, base.y);
      ctx.lineTo(top.x, top.y);
      ctx.stroke();
    }

    function drawScenery(item) {
      if (item.type === "tower") {
        const baseY = item.y ?? groundY;
        const halfW = (item.w || 80) * 0.5;
        const leftBase = projectPoint(item.x - halfW, baseY, item.z);
        const rightBase = projectPoint(item.x + halfW, baseY, item.z);
        const leftTop = projectPoint(item.x - halfW, baseY + (item.h || 420), item.z);
        const rightTop = projectPoint(item.x + halfW, baseY + (item.h || 420), item.z);
        if (!leftBase || !rightBase || !leftTop || !rightTop || leftBase.z > 3600) return;
        const base = projectPoint(item.x, baseY, item.z);
        if (!base) return;
        const width = Math.max(5, Math.hypot(rightBase.x - leftBase.x, rightBase.y - leftBase.y));
        const glow = item.glow || 0.18;
        ctx.fillStyle = item.hit ? "rgba(239,68,68,.24)" : `rgba(120, 156, 188, ${0.22 + glow})`;
        ctx.beginPath();
        ctx.moveTo(leftBase.x, leftBase.y);
        ctx.lineTo(rightBase.x, rightBase.y);
        ctx.lineTo(rightTop.x, rightTop.y);
        ctx.lineTo(leftTop.x, leftTop.y);
        ctx.closePath();
        ctx.fill();

        const inset = 0.18;
        const mix = (a, b, t) => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
        const innerLeftBase = mix(leftBase, rightBase, inset);
        const innerRightBase = mix(rightBase, leftBase, inset);
        const innerLeftTop = mix(leftTop, rightTop, inset);
        const innerRightTop = mix(rightTop, leftTop, inset);
        ctx.fillStyle = item.hit ? "rgba(254,202,202,.20)" : `rgba(210, 228, 244, ${0.18 + glow * 0.5})`;
        ctx.beginPath();
        ctx.moveTo(innerLeftBase.x, innerLeftBase.y);
        ctx.lineTo(innerRightBase.x, innerRightBase.y);
        ctx.lineTo(innerRightTop.x, innerRightTop.y);
        ctx.lineTo(innerLeftTop.x, innerLeftTop.y);
        ctx.closePath();
        ctx.fill();

        ctx.strokeStyle = item.hit ? "rgba(254,202,202,.45)" : "rgba(236, 246, 255, 0.44)";
        ctx.lineWidth = Math.max(1, 1.6 * base.s);
        ctx.beginPath();
        ctx.moveTo(leftBase.x, leftBase.y);
        ctx.lineTo(rightBase.x, rightBase.y);
        ctx.lineTo(rightTop.x, rightTop.y);
        ctx.lineTo(leftTop.x, leftTop.y);
        ctx.closePath();
        ctx.stroke();

        const heightScreen = Math.max(10, Math.hypot(leftBase.x - leftTop.x, leftBase.y - leftTop.y));
        const windowRows = Math.min(18, Math.max(4, Math.floor(heightScreen / 18)));
        const windowCols = Math.min(5, Math.max(2, Math.floor(width / 12)));
        ctx.fillStyle = item.hit ? "rgba(254,226,226,.30)" : "rgba(250, 244, 169, 0.24)";
        for (let row = 0; row < windowRows; row += 1) {
          const rowT = (row + 0.5) / windowRows;
          const rowLeft = mix(innerLeftTop, innerLeftBase, rowT);
          const rowRight = mix(innerRightTop, innerRightBase, rowT);
          for (let col = 0; col < windowCols; col += 1) {
            const colT = windowCols === 1 ? 0.5 : col / (windowCols - 1);
            const px = rowLeft.x + (rowRight.x - rowLeft.x) * colT;
            const py = rowLeft.y + (rowRight.y - rowLeft.y) * colT;
            ctx.beginPath();
            ctx.arc(px, py, Math.max(1.2, 2.2 * base.s), 0, Math.PI * 2);
            ctx.fill();
          }
        }
        return;
      }

      const base = projectPoint(item.x, -110, item.z);
      if (!base || base.z > 2800) return;
      const h = (item.h || (item.type === "tree" ? 46 : 16)) * base.s;
      const w = (item.type === "tree" ? 18 : 10) * base.s;
      const angle = -game.roll * rollVisual;
      const topX = base.x + Math.sin(angle) * h * 0.45;
      const topY = base.y - Math.cos(angle) * h;
      ctx.strokeStyle = item.type === "tree" ? "#22c55e" : "#84cc16";
      ctx.lineWidth = Math.max(1, w);
      ctx.beginPath();
      ctx.moveTo(base.x, base.y);
      ctx.lineTo(topX, topY);
      ctx.stroke();
      if (item.type === "tree") {
        ctx.fillStyle = "rgba(34,197,94,.35)";
        ctx.beginPath();
        ctx.arc(topX, topY, Math.max(2, 15 * base.s), 0, Math.PI * 2);
        ctx.fill();
      }
    }

    function drawCourseMarker(marker) {
      const p = projectPoint(marker.x, marker.y, marker.z);
      if (!p || p.z > 3200) return;
      const r = Math.max(3, 24 * p.s);
      ctx.strokeStyle = "rgba(226,232,240,.24)";
      ctx.lineWidth = Math.max(1, 1.8 * p.s);
      ctx.beginPath();
      ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      ctx.stroke();
    }

    function landingWorldPoint(side, forward) {
      const base = game.landing || { x: 0, z: game.planeZ, heading: game.heading };
      const sin = Math.sin(base.heading);
      const cos = Math.cos(base.heading);
      return {
        x: base.x + sin * forward + cos * side,
        z: base.z + cos * forward - sin * side,
      };
    }

    function drawRunway() {
      if (!game || !["countdown", "landing"].includes(game.phase)) return;
      const segments = game.phase === "landing"
        ? [[120, 2100], [2100, 2500]]
        : [[-420, 1120]];
      for (const [a, b] of segments) {
        const l1 = landingWorldPoint(-95, a);
        const r1 = landingWorldPoint(95, a);
        const l2 = landingWorldPoint(-95, b);
        const r2 = landingWorldPoint(95, b);
        fillPoly(
          projectPoint(l1.x, -108, l1.z),
          projectPoint(r1.x, -108, r1.z),
          projectPoint(r2.x, -108, r2.z),
          projectPoint(l2.x, -108, l2.z),
          "rgba(15,23,42,.52)"
        );
        const c1 = landingWorldPoint(0, a);
        const c2 = landingWorldPoint(0, b);
        drawLine(projectPoint(c1.x, -106, c1.z), projectPoint(c2.x, -106, c2.z), "rgba(250,204,21,.45)", 2);
      }
      for (let f = game.phase === "landing" ? 160 : -360; f <= (game.phase === "landing" ? 2380 : 1040); f += 120) {
        for (const side of [-118, 118]) {
          const light = landingWorldPoint(side, f);
          const p = projectPoint(light.x, -102, light.z);
          if (!p) continue;
          const r = Math.max(2, 10 * p.s);
          ctx.fillStyle = side < 0 ? "rgba(56,189,248,.72)" : "rgba(250,204,21,.72)";
          ctx.beginPath();
          ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      if (game.phase === "landing") {
        ctx.save();
        ctx.textAlign = "center";
        ctx.font = "20px DotGothic16";
        ctx.fillStyle = "#fef3c7";
        ctx.textBaseline = "middle";
        ctx.fillText(t("landingBanner"), centerX, Math.max(72, H * 0.17));
        ctx.restore();
      }
    }

    function drawStartSignal() {
      if (!game || game.phase !== "countdown") return;
      const elapsed = (performance.now() - game.countdownStartMs) / 1000;
      const lit = Math.min(4, Math.floor(elapsed) + 1);
      const labels = ["●", "●", "●", "●"];
      const briefing = roundBriefing(game.round);
      const level = Math.max(1, Math.min(10, briefing.difficulty || 1));
      const stars = `LV ${level}`;
      ctx.save();
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      const panelW = Math.min(W - 28, 388);
      const panelH = 132;
      const panelX = centerX - panelW / 2;
      const panelY = Math.max(48, H * 0.14);
      ctx.fillStyle = "rgba(2, 6, 23, .82)";
      ctx.strokeStyle = "rgba(125, 249, 255, .42)";
      ctx.lineWidth = 2;
      roundRect(panelX, panelY, panelW, panelH, 14);
      ctx.fill();
      ctx.stroke();
      ctx.shadowColor = "rgba(56, 189, 248, 0.35)";
      ctx.shadowBlur = 14;
      ctx.font = "17px DotGothic16";
      ctx.fillStyle = "#e0f2fe";
      ctx.fillText(briefing.title, centerX, panelY + 28);
      ctx.font = "22px DotGothic16";
      ctx.fillStyle = "#facc15";
      ctx.fillText(stars, centerX, panelY + 58);
      ctx.font = "12px DotGothic16";
      ctx.fillStyle = "#bae6fd";
      wrapCenterText(briefing.text, centerX, panelY + 88, panelW - 30, 17);
      ctx.shadowBlur = 22;
      ctx.font = "52px DotGothic16";
      for (let i = 0; i < 4; i += 1) {
        ctx.fillStyle = i < lit ? (i < 3 ? "#ef4444" : "#22c55e") : "rgba(148,163,184,.35)";
        ctx.fillText(labels[i], centerX - 84 + i * 56, panelY + panelH + 48);
      }
      ctx.shadowColor = lit >= 4 ? "rgba(34, 197, 94, 0.45)" : "rgba(248, 113, 113, 0.35)";
      ctx.shadowBlur = 18;
      ctx.font = "24px DotGothic16";
      ctx.fillStyle = lit >= 4 ? "#bbf7d0" : "#fecaca";
      ctx.fillText(lit >= 4 ? t("go") : t("ready"), centerX, panelY + panelH + 96);
      ctx.restore();
    }

    function roundRect(x, y, w, h, r) {
      const rr = Math.min(r, w / 2, h / 2);
      ctx.beginPath();
      ctx.moveTo(x + rr, y);
      ctx.lineTo(x + w - rr, y);
      ctx.quadraticCurveTo(x + w, y, x + w, y + rr);
      ctx.lineTo(x + w, y + h - rr);
      ctx.quadraticCurveTo(x + w, y + h, x + w - rr, y + h);
      ctx.lineTo(x + rr, y + h);
      ctx.quadraticCurveTo(x, y + h, x, y + h - rr);
      ctx.lineTo(x, y + rr);
      ctx.quadraticCurveTo(x, y, x + rr, y);
      ctx.closePath();
    }

    function wrapCenterText(text, x, y, maxWidth, lineHeight) {
      let line = "";
      let lineY = y;
      for (const char of text) {
        const test = line + char;
        if (line && ctx.measureText(test).width > maxWidth) {
          ctx.fillText(line, x, lineY);
          line = char;
          lineY += lineHeight;
        } else {
          line = test;
        }
      }
      if (line) ctx.fillText(line, x, lineY);
    }

    function drawPlaneHud() {
      const aircraft = game.aircraft || AIRCRAFTS[0];
      ctx.save();
      ctx.translate(centerX, H * 0.66);
      ctx.rotate(game.roll * 0.62);
      drawBoostEffect(ctx, aircraft, 1, 0.45 + input.throttle * 0.7);
      drawAircraftShape(aircraft, 1, aircraft.color, aircraft.stroke);
      ctx.restore();

      ctx.strokeStyle = "rgba(255,255,255,.65)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(centerX - 42, centerY);
      ctx.lineTo(centerX - 12, centerY);
      ctx.moveTo(centerX + 12, centerY);
      ctx.lineTo(centerX + 42, centerY);
      ctx.stroke();
    }

    function drawAircraftShape(aircraft, scaleMul, fill, stroke) {
      drawAircraftShapeTo(ctx, aircraft, scaleMul, fill, stroke);
    }

    function drawBoostEffect(targetCtx, aircraft, scaleMul, intensity) {
      const shape = aircraft?.shape || "standard";
      const power = Math.max(0.15, Math.min(1.2, intensity));
      const unit = 22 * scaleMul;
      const tailWidth = shape === "broad" ? 0.62 : shape === "heavy" ? 0.68 : shape === "fang" ? 0.42 : 0.52;
      const tailLength = shape === "dart" ? 2.1 : shape === "heavy" ? 1.45 : shape === "fang" ? 1.75 : 1.6;
      const spread = shape === "broad" ? 0.76 : shape === "heavy" ? 0.58 : 0.48;
      targetCtx.save();
      targetCtx.globalCompositeOperation = "screen";
      const alpha = 0.18 + power * 0.24;
      targetCtx.fillStyle = `rgba(255, 241, 118, ${alpha})`;
      targetCtx.beginPath();
      targetCtx.moveTo(-tailWidth * unit, 0.70 * unit);
      targetCtx.quadraticCurveTo(0, (tailLength + 0.35 * power) * unit, tailWidth * unit, 0.70 * unit);
      targetCtx.closePath();
      targetCtx.fill();
      targetCtx.fillStyle = `rgba(125, 211, 252, ${0.16 + power * 0.18})`;
      targetCtx.beginPath();
      targetCtx.moveTo(-spread * unit, 0.62 * unit);
      targetCtx.quadraticCurveTo(0, (tailLength + 0.85 * power) * unit, spread * unit, 0.62 * unit);
      targetCtx.closePath();
      targetCtx.fill();
      targetCtx.restore();
    }

    function drawRivals() {
      if (!game.rivals || game.rivals.length === 0) return;
      const drawn = [];
      for (let i = 0; i < game.rivals.length; i += 1) {
        const rival = game.rivals[i];
        if (!rival || rival.finished) continue;
        const heightOffset = game.phase === "countdown" ? 28 : 32;
        const p = projectPoint(rival.x, rival.y + heightOffset, rival.z);
        if (!p || p.z > 3000) continue;
        const size = Math.max(4, 24 * p.s);
        const aircraft = rival.aircraft || AIRCRAFTS[(i + 1) % AIRCRAFTS.length];
        ctx.save();
        ctx.translate(p.x, p.y);
        ctx.rotate(rival.heading - game.heading - game.roll * 0.2);
        drawBoostEffect(ctx, aircraft, size / 22, rival.kind === "ghost" ? 0.42 : 0.68);
        drawAircraftShape(aircraft, size / 22, aircraft.color, aircraft.stroke);
        ctx.restore();
        drawn.push({ rival, aircraft, p, size });
      }
      drawn.sort((a, b) => b.size - a.size);
      const labelLimit = currentFieldSize() <= 5 ? drawn.length : Math.min(4, drawn.length);
      for (let i = 0; i < labelLimit; i += 1) {
        const { rival, aircraft, p, size } = drawn[i];
        const isGhost = rival.kind === "ghost";
        ctx.fillStyle = isGhost ? "rgba(191, 219, 254, 0.96)" : "rgba(254, 202, 202, 0.96)";
        ctx.font = `${Math.max(9, 10 * p.s + 1)}px DotGothic16`;
        ctx.textAlign = "center";
        const aircraftLabel = currentLang === "ja"
          ? aircraft.name.replace("号", "")
          : (aircraft.nameEn || aircraft.name);
        const typeLabel = isGhost ? t("ghost") : t("cpu");
        ctx.fillText(`[${typeLabel}] ${rival.name} / ${aircraftLabel}`, p.x, p.y - size - 8);
      }
    }

    function drawDamage() {
      if (game.damage <= 0) return;
      ctx.fillStyle = "rgba(239,68,68,.10)";
      ctx.fillRect(0, 0, W, H);
    }

    function drawSpeedGauge() {
      const x = centerX;
      const y = H - 88;
      const r = 38;
      const pct = Math.max(0, Math.min(1, game.speed / maxBoostSpeed));
      const start = Math.PI * 0.78;
      const end = Math.PI * 2.22;
      const angle = start + (end - start) * pct;
      ctx.save();
      ctx.translate(x, y);
      ctx.strokeStyle = "rgba(186,230,253,.55)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(0, 0, r, start, end);
      ctx.stroke();
      for (let i = 0; i <= 10; i += 1) {
        const a = start + (end - start) * (i / 10);
        const inner = i % 5 === 0 ? r - 8 : r - 5;
        ctx.beginPath();
        ctx.moveTo(Math.cos(a) * inner, Math.sin(a) * inner);
        ctx.lineTo(Math.cos(a) * r, Math.sin(a) * r);
        ctx.stroke();
      }
      ctx.strokeStyle = "#facc15";
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(Math.cos(angle) * (r - 10), Math.sin(angle) * (r - 10));
      ctx.stroke();
      ctx.fillStyle = "#e0f2fe";
      ctx.font = "10px DotGothic16";
      ctx.textAlign = "center";
      ctx.fillText("SPEED", 0, 10);
      ctx.font = "12px DotGothic16";
      ctx.fillText(`${Math.round(game.speed)} km/h`, 0, 25);
      ctx.restore();
    }

    function drawTopSpeedText() {
      ctx.save();
      ctx.textAlign = "center";
      ctx.fillStyle = "#f8fafc";
      ctx.shadowColor = "rgba(56, 189, 248, 0.32)";
      ctx.shadowBlur = 14;
      ctx.font = "34px DotGothic16";
      ctx.fillText(`${Math.round(game.speed)}`, centerX, 40);
      ctx.font = "12px DotGothic16";
      ctx.fillStyle = "#93c5fd";
      ctx.fillText("km/h", centerX, 58);
      ctx.restore();
    }

    function drawAltTape() {
      const x = W - 28;
      const top = 80;
      const bottom = H - 132;
      const h = bottom - top;
      const alt = game.altitude;
      ctx.strokeStyle = "rgba(186,230,253,.55)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(x, top);
      ctx.lineTo(x, bottom);
      ctx.stroke();
      for (let a = -200; a <= 1200; a += 50) {
        const y = top + h / 2 - (a - alt) * 0.34;
        if (y < top || y > bottom) continue;
        const major = a % 100 === 0;
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x - (major ? 18 : 10), y);
        ctx.stroke();
        if (major) {
          ctx.fillStyle = "#bae6fd";
          ctx.font = "10px DotGothic16";
          ctx.textAlign = "right";
          ctx.fillText(`${a}`, x - 22, y + 3);
        }
      }
      ctx.fillStyle = "rgba(250,204,21,.90)";
      ctx.beginPath();
      ctx.moveTo(x - 26, top + h / 2);
      ctx.lineTo(x - 12, top + h / 2 - 8);
      ctx.lineTo(x - 12, top + h / 2 + 8);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = "#fef3c7";
      ctx.font = "11px DotGothic16";
      ctx.textAlign = "right";
      ctx.fillText(`${Math.round(alt)}m`, x - 30, top + h / 2 + 4);
    }

    function drawMiniMap() {
      if (!game?.coursePath || game.coursePath.length < 2) return;
      const panelW = 112;
      const panelH = 136;
      const x = 12;
      const y = 14;
      const padding = 10;
      const mapW = panelW - padding * 2;
      const mapH = panelH - padding * 2 - 20;
      const bounds = game.courseBounds || { minX: -1000, maxX: 1000, minZ: -1000, maxZ: 1000 };
      const spanX = Math.max(1, bounds.maxX - bounds.minX);
      const spanZ = Math.max(1, bounds.maxZ - bounds.minZ);
      const scale = Math.min(mapW / spanX, mapH / spanZ);
      const offsetX = x + padding + (mapW - spanX * scale) * 0.5;
      const offsetY = y + padding + 28 + (mapH - spanZ * scale) * 0.5;
      const toMap = (wx, wz) => ({
        x: offsetX + (wx - bounds.minX) * scale,
        y: offsetY + (bounds.maxZ - wz) * scale,
      });

      ctx.save();
      ctx.fillStyle = "rgba(2, 6, 23, 0.66)";
      ctx.strokeStyle = "rgba(125, 249, 255, 0.32)";
      ctx.lineWidth = 1;
      roundRect(x, y, panelW, panelH, 10);
      ctx.fill();
      ctx.stroke();

      const briefing = roundBriefing(game.round);
      const title = briefing?.title || `ROUND ${game.round}`;
      const parts = title.split(" / ");
      const roundLabel = parts[0] || `ROUND ${game.round}`;
      const placeLabel = (parts[1] || "").split(" ")[0];
      ctx.fillStyle = "#bae6fd";
      ctx.font = "13px DotGothic16";
      ctx.textAlign = "left";
      ctx.fillText(roundLabel, x + padding, y + 15);
      if (placeLabel) {
        ctx.fillStyle = "#fef3c7";
        ctx.font = "12px DotGothic16";
        ctx.fillText(placeLabel, x + padding, y + 30);
      }

      ctx.strokeStyle = "rgba(103, 232, 249, 0.44)";
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      game.coursePath.forEach((point, index) => {
        const p = toMap(point.x, point.z);
        if (index === 0) ctx.moveTo(p.x, p.y);
        else ctx.lineTo(p.x, p.y);
      });
      ctx.stroke();

      ctx.fillStyle = "rgba(226, 232, 240, 0.72)";
      game.gates.forEach((gate, index) => {
        const p = toMap(gate.x, gate.z);
        ctx.fillStyle = gate.passed ? "rgba(250, 204, 21, 0.78)" : "rgba(226, 232, 240, 0.72)";
        const r = index === game.gateIndex ? 3 : 2;
        ctx.beginPath();
        ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
        ctx.fill();
      });

      for (const rival of game.rivals || []) {
        if (!rival || rival.finished) continue;
        const p = toMap(rival.x, rival.z);
        ctx.fillStyle = rival.kind === "ghost" ? "rgba(147, 197, 253, 0.88)" : "rgba(248, 113, 113, 0.88)";
        ctx.beginPath();
        ctx.arc(p.x, p.y, 2.6, 0, Math.PI * 2);
        ctx.fill();
      }

      const player = toMap(game.planeX, game.planeZ);
      ctx.fillStyle = "#22c55e";
      ctx.beginPath();
      ctx.arc(player.x, player.y, 3.4, 0, Math.PI * 2);
      ctx.fill();

      const hx = player.x + Math.sin(game.heading) * 7;
      const hy = player.y - Math.cos(game.heading) * 7;
      ctx.strokeStyle = "rgba(220, 252, 231, 0.92)";
      ctx.lineWidth = 1.4;
      ctx.beginPath();
      ctx.moveTo(player.x, player.y);
      ctx.lineTo(hx, hy);
      ctx.stroke();
      ctx.restore();
    }

    function participantEntries() {
      const entries = [];
      const gate = game.gates[game.gateIndex];
      const playerDist = gate ? distance2d(gate.x, gate.z, game.planeX, game.planeZ) : 0;
      const playerProgress = racerProgressValue(game.gateIndex, playerDist);
      entries.push({
        label: t("you"),
        kind: "player",
        progress: playerProgress,
        gatePassed: game.phase === "landing" ? game.gates.length : game.gateIndex,
        finished: false,
        dnf: false,
      });
      for (const rival of game.rivals || []) {
        const dnf = rival.kind === "ghost" && rival.status === "dnf" && rival.finished;
        const gatePassed = rival.progress >= game.gates.length * 100000
          ? game.gates.length
          : Math.max(0, Math.min(game.gates.length, Math.floor((rival.progress || 0) / 100000)));
        entries.push({
          label: rival.name,
          kind: rival.kind,
          progress: dnf ? -1e12 : (rival.progress || 0),
          gatePassed,
          finished: !!rival.finished,
          dnf,
        });
      }
      entries.sort((a, b) => b.progress - a.progress);
      return entries.map((entry, index) => ({ ...entry, position: index + 1 }));
    }

    function drawParticipantBoard() {
      const entries = participantEntries();
      if (!entries.length) return;
      const x = 12;
      const y = 156;
      const w = 112;
      const lineH = entries.length >= 10 ? 10 : 12;
      ctx.save();
      ctx.textAlign = "left";
      ctx.font = `${entries.length >= 10 ? 9 : 10}px DotGothic16`;
      ctx.lineJoin = "round";
      ctx.lineWidth = 3;
      entries.forEach((entry, index) => {
        const py = y + 12 + index * lineH;
        if (py > H - 12) return;
        const isYou = entry.kind === "player";
        const labelColor = isYou
          ? "#fef08a"
          : entry.kind === "ghost"
            ? "#bfdbfe"
            : "#fecaca";
        ctx.strokeStyle = isYou ? "rgba(20, 16, 4, 0.96)" : "rgba(2, 6, 23, 0.92)";
        ctx.fillStyle = labelColor;
        const name = entry.label.length > 10 ? `${entry.label.slice(0, 10)}` : entry.label;
        const line = isYou
          ? `>${entry.position}. ${t("you")} G${entry.gatePassed}<`
          : `${entry.position}. ${name} G${entry.gatePassed}`;
        if (isYou) {
          ctx.lineWidth = 4;
        } else {
          ctx.lineWidth = 3;
        }
        ctx.strokeText(line, x + 6, py);
        ctx.fillText(line, x + 6, py);
        if (entry.dnf) {
          ctx.textAlign = "right";
          ctx.strokeStyle = "rgba(2, 6, 23, 0.92)";
          ctx.fillStyle = "#facc15";
          ctx.strokeText("DNF", x + w - 2, py);
          ctx.fillText("DNF", x + w - 2, py);
          ctx.textAlign = "left";
        }
      });
      ctx.restore();
    }

    function drawBottomHudText() {
      const snapshot = game.hudSnapshot || {};
      const fieldSize = snapshot.fieldSize || currentFieldSize();
      const gateLabel = game.phase === "landing"
        ? t("runway")
        : t("gate", { index: Math.min(game.gateIndex + 1, game.gates.length) });
      const primary = game.phase === "landing"
        ? "FINAL APPROACH"
        : `${gateLabel} / POS ${game.racePosition || 1}/${fieldSize} / GAP ${Math.round((snapshot.progressGap || 0) * 0.01)}`;

      ctx.save();
      ctx.textAlign = "center";
      ctx.fillStyle = "#dff9ff";
      ctx.font = "12px DotGothic16";
      ctx.fillText(primary, centerX, H - 20);
      ctx.restore();
    }

    function drawRightHudInfo() {
      const snapshot = game.hudSnapshot || {};
      const lines = [
        `ROUND ${game.round}`,
        `TIME ${snapshot.elapsed?.toFixed(2) || "0.00"}`,
        `DAMAGE ${Math.round(game.damage)}%`,
      ];
      if (game.round >= GRAND_PRIX_START_ROUND && game.tournament) {
        lines.push(`GP ${game.tournament.playerPoints}pt`);
      }

      ctx.save();
      ctx.textAlign = "right";
      ctx.fillStyle = "#e0f2fe";
      ctx.font = "12px DotGothic16";
      lines.forEach((line, index) => {
        ctx.fillText(line, W - 14, 18 + index * 15);
      });
      ctx.restore();
    }

    function drawHud() {
      drawTopSpeedText();
      drawSpeedGauge();
      drawAltTape();
      drawMiniMap();
      drawParticipantBoard();
      drawBottomHudText();
      drawRightHudInfo();
    }

    function update(dt) {
      const aircraft = game.aircraft || AIRCRAFTS[0];
      applyTiltInput();
      applyGamepadInput();
      if (controlMode !== "tilt" && controlMode !== "gamepad" && stickPointer === null) {
        input.x += (keyboardTarget.x - input.x) * Math.min(1, dt * 4.0);
        input.y += (keyboardTarget.y - input.y) * Math.min(1, dt * 5.0);
      }
      if (game.phase === "countdown") {
        const elapsed = (performance.now() - game.countdownStartMs) / 1000;
        game.roll += (0 - game.roll) * Math.min(1, dt * 4);
        updateCamera(dt);
        if (elapsed >= 3.2) {
          game.phase = "race";
          game.totalStartMs = performance.now();
          game.roundStartMs = game.totalStartMs;
          game.lastMs = game.totalStartMs;
          game.speed = 220;
          game.altitude = 34;
          ui.guide.style.display = "none";
          hitSound("gate");
          setMessage(t("greenBoost"), 1300);
        }
        return;
      }
      const turnRate = (1.65 - Math.min(0.55, Math.max(0, game.speed - 320) * 0.001)) * aircraft.turnMul;
      game.heading += input.x * turnRate * dt;
      game.roll += (input.x * 1.45 - game.roll) * Math.min(1, dt * 5.8);

      const throttle = Math.max(input.boost ? 1 : 0, input.throttle || 0);
      if (controlMode === "gamepad") {
        const accel = 430 * throttle - 135 * (1 - throttle);
        game.speed = Math.max(180, Math.min(maxBoostSpeed * aircraft.boostMul, game.speed + accel * aircraft.speedMul * dt));
      } else {
        const targetSpeed = cruiseSpeed * aircraft.speedMul + (maxBoostSpeed * aircraft.boostMul - cruiseSpeed * aircraft.speedMul) * throttle;
        game.speed += (targetSpeed - game.speed) * Math.min(1, dt * 1.8);
      }
      const assistActive = performance.now() < (game.takeoffAssistUntil || 0);
      const climbInput = assistActive ? 0 : input.y;
      game.altitude += climbInput * 175 * aircraft.climbMul * dt;
      if (assistActive) {
        const assistTargetAlt = 86;
        game.altitude += Math.max(0, assistTargetAlt - game.altitude) * Math.min(1, dt * 1.8);
      }
      game.altitude = Math.max(minAlt, Math.min(maxAlt, game.altitude));
      updateCamera(dt);
      game.distance += game.speed * dt;
      game.planeX += Math.sin(game.heading) * game.speed * dt;
      game.planeZ += Math.cos(game.heading) * game.speed * dt;
      updateRivals();
      game.racePosition = currentRacePosition();
      updateAudio();

      if (game.phase === "landing") {
        updateLanding();
        return;
      }

      const gate = game.gates[game.gateIndex];
      if (gate && distance2d(gate.x, gate.z, game.planeX, game.planeZ) < 132) {
        const dx = Math.abs(lateralError(gate));
        const dy = Math.abs(gate.y - playerWorldY());
        if (dx < 118 && dy < 98) {
          gate.passed = true;
          game.gateIndex += 1;
          hitSound("gate");
          setMessage(t("gateClear", { index: game.gateIndex }), 900);
        } else {
          game.damage = Math.min(100, game.damage + 12);
          game.gateIndex += 1;
          hitSound("pylon");
          setMessage(t("gateMiss"), 1200);
        }
      }

      for (const pylon of game.pylons) {
        if (pylon.hit) continue;
        if (distance2d(pylon.x, pylon.z, game.planeX, game.planeZ) < 44) {
          const dx = Math.abs(pylon.x - game.planeX);
          const dy = Math.abs((pylon.y + pylon.h * 0.5) - playerWorldY());
          const hitWidth = pylon.gateEdge ? 22 : 26;
          if (dx < hitWidth && dy < pylon.h * 0.58) {
            pylon.hit = true;
            game.damage = Math.min(100, game.damage + 18);
            hitSound("pylon");
            setMessage(t("pylonHit"), 1300);
          }
        }
      }

      for (const item of game.scenery || []) {
        if (item.type !== "tower" || item.hit) continue;
        const halfW = (item.w || 80) * 0.5;
        const halfD = (item.d || 80) * 0.5;
        const dx = Math.abs(item.x - game.planeX);
        const dz = Math.abs(item.z - game.planeZ);
        const dy = playerWorldY() - (item.y ?? groundY);
        if (dx < halfW + 18 && dz < halfD + 18 && dy > 0 && dy < (item.h || 420)) {
          item.hit = true;
          game.damage = Math.min(100, game.damage + 24);
          hitSound("pylon");
          setMessage(t("towerHit"), 1400);
        }
      }

      if (game.altitude < 45) {
        setMessage(t("altitudeWarn"), 500);
      }

      if (game.altitude <= minAlt + 1 && game.phase !== "countdown") {
        const nowMs = performance.now();
        if (!game.groundHitAt) {
          game.groundHitAt = nowMs;
          setMessage(t("groundWarning"), 500);
        } else if (nowMs - game.groundHitAt > 320) {
          hitSound("crash");
          setMessage(t("groundImpact"), 1200);
          finish(false, "crash");
          return;
        }
      } else {
        game.groundHitAt = null;
      }

      if (game.damage >= 100) {
        hitSound("crash");
        finish(false, "crash");
      } else if (game.gateIndex >= game.gates.length) {
        beginLanding();
      }
    }

    function racerProgressValue(gateIndex, dist) {
      return gateIndex * 100000 - dist;
    }

    function currentRacePosition(elapsedOverride = null) {
      if (elapsedOverride !== null && elapsedOverride !== undefined) {
        const playerTime = elapsedOverride;
        const faster = (game.rivals || []).filter((rival) => rival.status !== "dnf" && (rival.totalTime || Number.POSITIVE_INFINITY) < playerTime).length;
        return 1 + faster;
      }
      const playerProgress = game.playerProgress ?? 0;
      const values = [playerProgress, ...(game.rivals || []).map((rival) => rival.progress || 0)];
      values.sort((a, b) => b - a);
      return values.indexOf(playerProgress) + 1;
    }

    function updateRivals() {
      if (!game.rivals || game.rivals.length === 0 || game.phase === "countdown") return;
      const elapsed = (performance.now() - game.totalStartMs) / 1000;
      const nextGate = game.gates[Math.min(game.gateIndex, game.gates.length - 1)];
      const playerDist = nextGate ? distance2d(nextGate.x, nextGate.z, game.planeX, game.planeZ) : 0;
      game.playerProgress = game.phase === "landing"
        ? game.gates.length * 100000 + Math.max(0, landingProgress()) * 100
        : racerProgressValue(game.gateIndex, playerDist);
      for (const rival of game.rivals) {
        if (rival.kind === "ghost") {
          const pos = sampleFromGhost(rival.samples, elapsed * 1000);
          if (!pos) continue;
          rival.x = pos.x + (rival.laneOffset || 0);
          rival.y = pos.y;
          rival.z = pos.z;
          rival.heading = pos.h;
          rival.roll = pos.r || 0;
          rival.finished = elapsed >= rival.totalTime;
          rival.progress = rival.status === "dnf" && rival.finished
            ? -1e12
            : rival.finished
              ? game.gates.length * 100000 + Math.max(0, elapsed - rival.totalTime) * 100
              : racerProgressValue(
                  Math.min(game.gates.length - 1, Math.floor((elapsed / Math.max(rival.totalTime, 0.1)) * game.gates.length)),
                  (1 - Math.min(1, elapsed / Math.max(rival.totalTime, 0.1))) * 1000
                );
        } else {
          const raceTime = rival.raceTime || rival.totalTime;
          const landingDuration = rival.landingDuration || 0;
          const courseEnd = game.coursePath[game.coursePath.length - 1];
          const baseT = Math.min(1, elapsed / Math.max(raceTime, 0.1));
          let t = baseT;
          if (rival.style === "speed") t = Math.min(1, Math.pow(baseT, 0.91) + Math.sin(baseT * Math.PI * 2.0) * 0.008);
          else if (rival.style === "line") t = Math.min(1, baseT + Math.sin(baseT * Math.PI * 1.4) * 0.018);
          else if (rival.style === "steady") t = Math.min(1, baseT + Math.sin(baseT * Math.PI) * 0.005);
          const lane = (rival.laneOffset || 0) + Math.sin(elapsed * 0.55 + (rival.seed || 0)) * (rival.lineWave || 0);
          if (elapsed < raceTime) {
            const pos = interpolatePath(game.coursePath, t);
            if (!pos) continue;
            const nx = Math.cos(pos.heading);
            const nz = -Math.sin(pos.heading);
            rival.x = pos.x + nx * lane;
            rival.y = pos.y;
            rival.z = pos.z + nz * lane;
            rival.heading = pos.heading;
            rival.finished = false;
            rival.progress = racerProgressValue(
              Math.min(game.gates.length - 1, Math.floor(t * game.gates.length)),
              (1 - t) * 1000
            );
          } else {
            const landingT = Math.min(1, (elapsed - raceTime) / Math.max(landingDuration, 0.1));
            const nx = Math.cos(courseEnd.heading);
            const nz = -Math.sin(courseEnd.heading);
            const distance = 2400 * landingT;
            rival.x = courseEnd.x + Math.sin(courseEnd.heading) * distance + nx * lane;
            rival.y = courseEnd.y + (22 - courseEnd.y) * landingT;
            rival.z = courseEnd.z + Math.cos(courseEnd.heading) * distance + nz * lane;
            rival.heading = courseEnd.heading;
            rival.finished = landingT >= 1;
            rival.progress = game.gates.length * 100000 + distance * 100;
          }
        }
      }
    }

    function recordGhostSample(now) {
      if (!game || game.phase === "countdown") return;
      const t = Math.round(now - game.totalStartMs);
      if (t - game.lastGhostSampleMs < 120) return;
      game.lastGhostSampleMs = t;
      game.ghostSamples.push({
        t,
        x: Number(game.planeX.toFixed(1)),
        y: Number((game.altitude - 150).toFixed(1)),
        z: Number(game.planeZ.toFixed(1)),
        h: Number(game.heading.toFixed(4)),
        r: Number(game.roll.toFixed(4)),
        s: Math.round(game.speed),
      });
    }

    function beginLanding() {
      if (game.phase === "landing") return;
      game.phase = "landing";
      game.landing = {
        x: game.planeX,
        z: game.planeZ,
        heading: game.heading,
      };
      setMessage(t("landingPrompt"), 2600);
    }

    function landingProgress() {
      const base = game.landing;
      const dx = game.planeX - base.x;
      const dz = game.planeZ - base.z;
      return dx * Math.sin(base.heading) + dz * Math.cos(base.heading);
    }

    function landingLateral() {
      const base = game.landing;
      const dx = game.planeX - base.x;
      const dz = game.planeZ - base.z;
      return dx * Math.cos(base.heading) - dz * Math.sin(base.heading);
    }

    function updateLanding() {
      const progress = landingProgress();
      const lateral = Math.abs(landingLateral());
      if (progress > 160 && game.altitude < 46 && lateral < 105 && game.speed < 620) {
        setMessage(t("landingOk"), 1200);
        finishRound();
        return;
      }
      if (progress > 2450 || game.altitude <= 1) {
        hitSound("crash");
        setMessage(t("failedLanding"), 1200);
        finish(false, "crash");
        return;
      }
      if (progress > 900 && game.speed > 620) {
        setMessage(t("tooFast"), 500);
      } else if (progress > 900 && lateral > 110) {
        setMessage(t("returnCenter"), 500);
      } else if (progress > 900 && game.altitude > 80) {
        setMessage(t("lowerAltitude"), 500);
      }
    }

    function finishRound() {
      if (game.round < TOTAL_ROUNDS) {
        finish(true, "round_clear");
        return;
      }
      finish(true, "finish");
    }

    function distance2d(ax, az, bx, bz) {
      return Math.hypot(ax - bx, az - bz);
    }

    function guideInfo() {
      if (game.phase === "landing") {
        return { arrow: "↓", dist: Math.max(0, 1600 - landingProgress()) };
      }
      const gate = game.gates[game.gateIndex];
      if (!gate) return null;
      const dx = gate.x - game.planeX;
      const dz = gate.z - game.planeZ;
      const forward = dx * Math.sin(game.heading) + dz * Math.cos(game.heading);
      const side = dx * Math.cos(game.heading) - dz * Math.sin(game.heading);
      const dist = Math.hypot(dx, dz);
      let arrow = "↑";
      if (forward < -80) {
        arrow = side >= 0 ? "↻" : "↺";
      } else if (side > 55) {
        arrow = "→";
      } else if (side < -55) {
        arrow = "←";
      }
      return { arrow, dist };
    }

    function lateralError(gate) {
      const dx = gate.x - game.planeX;
      const dz = gate.z - game.planeZ;
      return dx * Math.cos(game.heading) - dz * Math.sin(game.heading);
    }
