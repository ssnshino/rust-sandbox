    function selectedAircraft() {
      return AIRCRAFTS.find((plane) => plane.id === selectedAircraftId) || AIRCRAFTS[0];
    }

    function cycleAircraft(direction) {
      const index = Math.max(0, AIRCRAFTS.findIndex((plane) => plane.id === selectedAircraftId));
      const nextIndex = (index + direction + AIRCRAFTS.length) % AIRCRAFTS.length;
      selectedAircraftId = AIRCRAFTS[nextIndex].id;
      localStorage.setItem("airrace_aircraft", selectedAircraftId);
      renderPlaneSelect();
    }

    function renderPlaneSelect() {
      if (!ui.planeDisplay) return;
      const plane = selectedAircraft();
      if (ui.planeName) ui.planeName.textContent = aircraftDisplayName(plane);
      if (ui.planeSummary) ui.planeSummary.textContent = aircraftDisplaySummary(plane);
      renderPlanePreview(plane);
      renderFieldMode();
      renderStartRound();
    }

    function drawAircraftShapeTo(targetCtx, aircraft, scaleMul, fill, stroke) {
      const shape = aircraft?.shape || "standard";
      const unit = 22 * scaleMul;
      targetCtx.fillStyle = fill;
      targetCtx.strokeStyle = stroke;
      targetCtx.lineWidth = Math.max(1.3, 2 * scaleMul);

      if (shape === "broad") {
        targetCtx.beginPath();
        targetCtx.moveTo(0, -0.95 * unit);
        targetCtx.lineTo(1.75 * unit, 0.26 * unit);
        targetCtx.lineTo(0.52 * unit, 0.16 * unit);
        targetCtx.lineTo(0.20 * unit, 0.96 * unit);
        targetCtx.lineTo(-0.20 * unit, 0.96 * unit);
        targetCtx.lineTo(-0.52 * unit, 0.16 * unit);
        targetCtx.lineTo(-1.75 * unit, 0.26 * unit);
        targetCtx.closePath();
      } else if (shape === "dart") {
        targetCtx.beginPath();
        targetCtx.moveTo(0, -1.25 * unit);
        targetCtx.lineTo(0.78 * unit, -0.12 * unit);
        targetCtx.lineTo(1.10 * unit, 0.56 * unit);
        targetCtx.lineTo(0.18 * unit, 0.20 * unit);
        targetCtx.lineTo(0, 1.00 * unit);
        targetCtx.lineTo(-0.18 * unit, 0.20 * unit);
        targetCtx.lineTo(-1.10 * unit, 0.56 * unit);
        targetCtx.lineTo(-0.78 * unit, -0.12 * unit);
        targetCtx.closePath();
      } else if (shape === "fang") {
        targetCtx.beginPath();
        targetCtx.moveTo(0, -1.05 * unit);
        targetCtx.lineTo(1.38 * unit, 0.02 * unit);
        targetCtx.lineTo(0.44 * unit, 0.22 * unit);
        targetCtx.lineTo(0.84 * unit, 0.92 * unit);
        targetCtx.lineTo(0.10 * unit, 0.42 * unit);
        targetCtx.lineTo(0, 1.04 * unit);
        targetCtx.lineTo(-0.10 * unit, 0.42 * unit);
        targetCtx.lineTo(-0.84 * unit, 0.92 * unit);
        targetCtx.lineTo(-0.44 * unit, 0.22 * unit);
        targetCtx.lineTo(-1.38 * unit, 0.02 * unit);
        targetCtx.closePath();
      } else if (shape === "heavy") {
        targetCtx.beginPath();
        targetCtx.moveTo(0, -0.88 * unit);
        targetCtx.lineTo(1.55 * unit, 0.32 * unit);
        targetCtx.lineTo(0.70 * unit, 0.52 * unit);
        targetCtx.lineTo(0.34 * unit, 1.12 * unit);
        targetCtx.lineTo(-0.34 * unit, 1.12 * unit);
        targetCtx.lineTo(-0.70 * unit, 0.52 * unit);
        targetCtx.lineTo(-1.55 * unit, 0.32 * unit);
        targetCtx.closePath();
      } else {
        targetCtx.beginPath();
        targetCtx.moveTo(0, -1.02 * unit);
        targetCtx.lineTo(1.46 * unit, 0.42 * unit);
        targetCtx.lineTo(0.30 * unit, 0.20 * unit);
        targetCtx.lineTo(0, 1.00 * unit);
        targetCtx.lineTo(-0.30 * unit, 0.20 * unit);
        targetCtx.lineTo(-1.46 * unit, 0.42 * unit);
        targetCtx.closePath();
      }
      targetCtx.fill();
      targetCtx.stroke();
    }

    function renderPlanePreview(aircraft) {
      if (!ui.planePreview) return;
      const previewCtx = ui.planePreview.getContext("2d");
      const w = ui.planePreview.width;
      const h = ui.planePreview.height;
      previewCtx.clearRect(0, 0, w, h);
      previewCtx.save();
      previewCtx.translate(w / 2, h / 2 + 2);
      previewCtx.strokeStyle = "rgba(186,230,253,.28)";
      previewCtx.lineWidth = 1;
      previewCtx.beginPath();
      previewCtx.moveTo(-84, 6);
      previewCtx.lineTo(-26, 6);
      previewCtx.moveTo(26, 6);
      previewCtx.lineTo(84, 6);
      previewCtx.stroke();
      drawBoostEffect(previewCtx, aircraft, 0.7, 0.85);
      drawAircraftShapeTo(previewCtx, aircraft, 0.85, aircraft.color, aircraft.stroke);
      previewCtx.restore();
    }

    function sampleFromGhost(samples, elapsedMs) {
      if (!samples || samples.length === 0) return null;
      if (elapsedMs <= samples[0].t) return samples[0];
      for (let i = 1; i < samples.length; i += 1) {
        const a = samples[i - 1];
        const b = samples[i];
        if (elapsedMs <= b.t) {
          const f = (elapsedMs - a.t) / Math.max(1, b.t - a.t);
          return {
            x: a.x + (b.x - a.x) * f,
            y: a.y + (b.y - a.y) * f,
            z: a.z + (b.z - a.z) * f,
            h: a.h + (b.h - a.h) * f,
            r: a.r + (b.r - a.r) * f,
            s: a.s + (b.s - a.s) * f,
          };
        }
      }
      return samples[samples.length - 1];
    }
