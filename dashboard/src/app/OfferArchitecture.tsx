import React from "react";

/**
 * Axonometric architecture plate for §2 · the offer.
 * Hand-tuned 2:1 isometric geometry — drafting-instrument restraint,
 * emerald/coral only where the system earns it. No filters, no glow.
 */
export default function OfferArchitecture() {
  // 2:1 isometric: sx = (x - y) * u, sy = (x + y) * (u/2) - z * u
  const u = 18;
  const iso = (x: number, y: number, z = 0) => ({
    x: (x - y) * u,
    y: (x + y) * (u / 2) - z * u,
  });
  const p = (x: number, y: number, z = 0) => {
    const pt = iso(x, y, z);
    return `${pt.x.toFixed(2)},${pt.y.toFixed(2)}`;
  };

  // Box faces from origin corner (x,y,z) with size (dx,dy,dz)
  const top = (x: number, y: number, z: number, dx: number, dy: number) =>
    `M ${p(x, y, z)} L ${p(x + dx, y, z)} L ${p(x + dx, y + dy, z)} L ${p(x, y + dy, z)} Z`;

  const left = (x: number, y: number, z: number, dy: number, dz: number) =>
    `M ${p(x, y, z)} L ${p(x, y + dy, z)} L ${p(x, y + dy, z - dz)} L ${p(x, y, z - dz)} Z`;

  const right = (x: number, y: number, z: number, dx: number, dz: number) =>
    `M ${p(x, y, z)} L ${p(x + dx, y, z)} L ${p(x + dx, y, z - dz)} L ${p(x, y, z - dz)} Z`;

  // Composition origin — shift into viewBox (lifted for vertical balance)
  const ox = 208;
  const oy = 248;

  // Volumes (in isometric grid units)
  // Base plinth
  const base = { x: 0, y: 0, z: 0.55, dx: 10.5, dy: 7.2, dz: 0.55 };
  // Client vault
  const vault = { x: 1.4, y: 1.1, z: 3.4, dx: 7.7, dy: 5.0, dz: 2.85 };
  // Gate stack (narrower tower)
  const gates = { x: 2.6, y: 2.0, z: 5.55, dx: 5.3, dy: 3.2, dz: 2.15 };
  // Mandate plate (thin top)
  const mandate = { x: 3.15, y: 2.4, z: 6.25, dx: 4.2, dy: 2.4, dz: 0.7 };

  // Vertical riser for kill-switch path (client-side, never enters vault authority)
  const riserX = -1.35;
  const riserY = 3.4;

  // Label anchors — kept clear of mass edges and plate footer
  const L = {
    mandate: iso(mandate.x + mandate.dx / 2, mandate.y - 0.55, mandate.z + 0.35),
    gates: iso(gates.x + gates.dx + 1.15, gates.y + 0.15, gates.z - 0.85),
    vault: iso(vault.x - 0.55, vault.y + vault.dy + 0.35, vault.z - 1.55),
    account: iso(base.x + base.dx / 2, base.y + base.dy + 0.95, 0.05),
    kill: iso(riserX - 0.4, riserY, 4.7),
    measure: iso(vault.x + vault.dx + 1.55, vault.y + 0.35, vault.z - 1.55),
  };

  return (
    <figure className="offer-arch" aria-label="Architecture: mandate through gates into a self-custodied vault">
      <svg
        className="offer-arch-svg"
        viewBox="0 0 420 340"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        role="img"
      >
        <title>Single-account autonomous system</title>
        <defs>
          {/* subtle grain via tiny dots — not a filter glow */}
          <pattern id="offer-hatch" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(30)">
            <line x1="0" y1="0" x2="0" y2="6" stroke="oklch(55% 0.1 160 / 0.07)" strokeWidth="1" />
          </pattern>
          <pattern id="offer-grid" width="18" height="18" patternUnits="userSpaceOnUse">
            <path
              d="M 18 0 L 0 0 0 18"
              fill="none"
              stroke="oklch(55% 0.1 160 / 0.06)"
              strokeWidth="0.6"
            />
          </pattern>
        </defs>

        <g transform={`translate(${ox} ${oy})`}>
          {/* Ground plane — quiet isometric diamond, not a neon pad */}
          <path
            className="oa-ground"
            d={`M ${p(-1.2, -1.2, 0)} L ${p(12.2, -1.2, 0)} L ${p(12.2, 8.6, 0)} L ${p(-1.2, 8.6, 0)} Z`}
            fill="url(#offer-grid)"
          />
          <path
            className="oa-ground-edge"
            d={`M ${p(-1.2, -1.2, 0)} L ${p(12.2, -1.2, 0)} L ${p(12.2, 8.6, 0)} L ${p(-1.2, 8.6, 0)} Z`}
          />

          {/* Contact shadow under plinth */}
          <ellipse
            className="oa-shadow"
            cx={iso(base.x + base.dx / 2, base.y + base.dy / 2, 0).x}
            cy={iso(base.x + base.dx / 2, base.y + base.dy / 2, 0).y + 6}
            rx={92}
            ry={22}
          />

          {/* ── Base plinth: your account ── */}
          <path className="oa-face oa-face-left" d={left(base.x, base.y, base.z, base.dy, base.dz)} />
          <path className="oa-face oa-face-right" d={right(base.x, base.y, base.z, base.dx, base.dz)} />
          <path className="oa-face oa-face-top oa-face-plinth" d={top(base.x, base.y, base.z, base.dx, base.dy)} />

          {/* ── Client vault ── */}
          <path className="oa-face oa-face-left" d={left(vault.x, vault.y, vault.z, vault.dy, vault.dz)} />
          <path className="oa-face oa-face-right" d={right(vault.x, vault.y, vault.z, vault.dx, vault.dz)} />
          <path className="oa-face oa-face-top" d={top(vault.x, vault.y, vault.z, vault.dx, vault.dy)} />
          {/* hatch on vault top — sealed volume */}
          <path className="oa-hatch" d={top(vault.x, vault.y, vault.z, vault.dx, vault.dy)} fill="url(#offer-hatch)" />

          {/* Internal vault seam lines (code authority — no keyhole) */}
          <path
            className="oa-seam"
            d={`M ${p(vault.x + 1.2, vault.y, vault.z)} L ${p(vault.x + 1.2, vault.y + vault.dy, vault.z)}`}
          />
          <path
            className="oa-seam"
            d={`M ${p(vault.x + vault.dx - 1.2, vault.y, vault.z)} L ${p(vault.x + vault.dx - 1.2, vault.y + vault.dy, vault.z)}`}
          />
          <path
            className="oa-seam"
            d={`M ${p(vault.x, vault.y + 1.15, vault.z)} L ${p(vault.x + vault.dx, vault.y + 1.15, vault.z)}`}
          />

          {/* Small recessed panel on vault right face — "code only" */}
          <path
            className="oa-inset"
            d={`M ${p(vault.x + 2.1, vault.y, vault.z - 0.7)}
                L ${p(vault.x + 5.6, vault.y, vault.z - 0.7)}
                L ${p(vault.x + 5.6, vault.y, vault.z - 2.0)}
                L ${p(vault.x + 2.1, vault.y, vault.z - 2.0)} Z`}
          />
          {/* three tick marks inside panel — not a lock icon */}
          {[0, 1, 2].map((i) => {
            const zz = vault.z - 1.0 - i * 0.35;
            return (
              <path
                key={i}
                className="oa-tick"
                d={`M ${p(vault.x + 2.55, vault.y, zz)} L ${p(vault.x + 5.15, vault.y, zz)}`}
              />
            );
          })}

          {/* ── Gate stack ── */}
          <path className="oa-face oa-face-left" d={left(gates.x, gates.y, gates.z, gates.dy, gates.dz)} />
          <path className="oa-face oa-face-right" d={right(gates.x, gates.y, gates.z, gates.dx, gates.dz)} />
          <path className="oa-face oa-face-top oa-face-gates" d={top(gates.x, gates.y, gates.z, gates.dx, gates.dy)} />

          {/* Horizontal gate slats on the right face */}
          {[0.35, 0.75, 1.15, 1.55].map((off, i) => (
            <path
              key={i}
              className="oa-gate-slat"
              d={`M ${p(gates.x, gates.y, gates.z - off)}
                  L ${p(gates.x + gates.dx, gates.y, gates.z - off)}`}
            />
          ))}

          {/* ── Mandate plate ── */}
          <path className="oa-face oa-face-left" d={left(mandate.x, mandate.y, mandate.z, mandate.dy, mandate.dz)} />
          <path className="oa-face oa-face-right" d={right(mandate.x, mandate.y, mandate.z, mandate.dx, mandate.dz)} />
          <path className="oa-face oa-face-top oa-face-mandate" d={top(mandate.x, mandate.y, mandate.z, mandate.dx, mandate.dy)} />

          {/* Thin constraint notches on mandate top */}
          {[0.7, 1.5, 2.3, 3.1].map((oxn, i) => (
            <path
              key={i}
              className="oa-notch"
              d={`M ${p(mandate.x + oxn, mandate.y + 0.45, mandate.z)}
                  L ${p(mandate.x + oxn, mandate.y + mandate.dy - 0.45, mandate.z)}`}
            />
          ))}

          {/* Vertical flow spine through centres */}
          <path
            className="oa-spine"
            d={`M ${p(mandate.x + mandate.dx / 2, mandate.y + mandate.dy / 2, mandate.z - mandate.dz)}
                L ${p(gates.x + gates.dx / 2, gates.y + gates.dy / 2, gates.z)}
                M ${p(gates.x + gates.dx / 2, gates.y + gates.dy / 2, gates.z - gates.dz)}
                L ${p(vault.x + vault.dx / 2, vault.y + vault.dy / 2, vault.z)}`}
          />

          {/* Measured-fee probe line from vault to right annotation */}
          <path
            className="oa-probe"
            d={`M ${p(vault.x + vault.dx, vault.y + 1.4, vault.z - 1.2)}
                L ${p(vault.x + vault.dx + 1.8, vault.y + 1.4, vault.z - 1.2)}`}
          />
          <circle
            className="oa-probe-dot"
            cx={iso(vault.x + vault.dx + 1.8, vault.y + 1.4, vault.z - 1.2).x}
            cy={iso(vault.x + vault.dx + 1.8, vault.y + 1.4, vault.z - 1.2).y}
            r="2.2"
          />

          {/* Kill-switch riser — client side, coral accent (only coral in the plate) */}
          <path
            className="oa-riser"
            d={`M ${p(riserX, riserY, 0.2)}
                L ${p(riserX, riserY, 5.1)}`}
          />
          <path
            className="oa-riser-arm"
            d={`M ${p(riserX, riserY, 5.1)}
                L ${p(mandate.x, riserY, 5.1)}`}
          />
          {/* small lever head */}
          <path
            className="oa-lever"
            d={`M ${p(riserX - 0.35, riserY, 5.1)}
                L ${p(riserX + 0.35, riserY, 5.1)}
                L ${p(riserX, riserY, 5.55)} Z`}
          />
          {/* dashed tether from riser base into plinth (you hold this) */}
          <path
            className="oa-tether"
            d={`M ${p(riserX, riserY, 0.2)}
                L ${p(base.x + 0.3, base.y + base.dy / 2, base.z)}`}
          />

          {/* Leader ticks — short hairlines from mass to labels */}
          <path
            className="oa-leader"
            d={`M ${p(gates.x + gates.dx, gates.y + 0.4, gates.z - 0.9)}
                L ${p(gates.x + gates.dx + 0.85, gates.y + 0.15, gates.z - 0.9)}`}
          />
          <path
            className="oa-leader"
            d={`M ${p(vault.x + vault.dx, vault.y + 1.1, vault.z - 1.35)}
                L ${p(vault.x + vault.dx + 1.1, vault.y + 0.7, vault.z - 1.35)}`}
          />

          {/* ── Labels ── */}
          <g className="oa-label" transform={`translate(${L.mandate.x} ${L.mandate.y - 16})`}>
            <text textAnchor="middle" className="oa-label-k">MANDATE</text>
            <text textAnchor="middle" y="11" className="oa-label-v">your constraints</text>
          </g>

          <g className="oa-label" transform={`translate(${L.gates.x + 6} ${L.gates.y - 2})`}>
            <text className="oa-label-k">GATES</text>
            <text y="11" className="oa-label-v">fixed suite</text>
          </g>

          <g className="oa-label" transform={`translate(${L.vault.x} ${L.vault.y + 4})`}>
            <text textAnchor="end" className="oa-label-k">VAULT</text>
            <text textAnchor="end" y="11" className="oa-label-v">code authority · no key</text>
          </g>

          <g className="oa-label" transform={`translate(${L.account.x} ${L.account.y + 8})`}>
            <text textAnchor="middle" className="oa-label-k oa-label-k-em">YOUR ACCOUNT</text>
            <text textAnchor="middle" y="11" className="oa-label-v">one destination</text>
          </g>

          <g className="oa-label oa-label-coral" transform={`translate(${L.kill.x - 4} ${L.kill.y})`}>
            <text textAnchor="end" className="oa-label-k">KILL SWITCH</text>
            <text textAnchor="end" y="11" className="oa-label-v">you hold it</text>
          </g>

          <g className="oa-label" transform={`translate(${L.measure.x + 8} ${L.measure.y})`}>
            <text className="oa-label-k">MEASURED</text>
            <text y="11" className="oa-label-v">on-chain fees</text>
          </g>

          {/* Registration marks — quiet drafting plate detail */}
          <g className="oa-reg" aria-hidden="true">
            <path d={`M ${p(-1.6, -1.6, 0)} l -7 0 M ${p(-1.6, -1.6, 0)} l 0 -7`} />
            <path d={`M ${p(12.6, -1.6, 0)} l 7 0 M ${p(12.6, -1.6, 0)} l 0 -7`} />
            <path d={`M ${p(-1.6, 9.0, 0)} l -7 0 M ${p(-1.6, 9.0, 0)} l 0 7`} />
            <path d={`M ${p(12.6, 9.0, 0)} l 7 0 M ${p(12.6, 9.0, 0)} l 0 7`} />
          </g>
        </g>

        {/* Plate index — top corners only; mass owns the lower field */}
        <text x="18" y="20" className="oa-plate-id">RTP-ARCH-01</text>
        <text x="402" y="20" textAnchor="end" className="oa-plate-id">ISO 2:1 · 1:1</text>
      </svg>
    </figure>
  );
}
