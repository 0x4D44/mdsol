# Windows Solitaire Victory Animation Research

## Historical notes
- Microsoft bundled Solitaire (sol.exe) with Windows 3.0 in 1990, and the celebratory cascade quickly became part of the OS' identity. ?cite?turn2search13?
- Creator Wes Cherry recalls tuning the finale for maximum speed, noting that publications even proposed timing the cascade as a makeshift benchmark. ?cite?turn3search1?
- Commentators later highlighted the bouncing-card finale as an early example of the satisfying "win state" feedback that would define many casual games. ?cite?turn8search2?

## How the original animation is triggered
- Sol.exe automatically runs the cascade the moment all four foundations hold complete Ace-through-King stacks. ?cite?turn8search2?
- Microsoft left in a keyboard shortcut—`Alt+Shift+2`—that forces the celebration at any time, a long-standing easter egg useful for testing. ?cite?turn0search5?

## Observed behaviour and inputs
- The animation renders each card as if it detaches from its pile, falls under gravity, rebounds off the window bounds, and trails across the playfield until it exits. ?cite?turn8search2?
- Mouse input modulates the cascade speed: moving the pointer across the screen during the sequence makes the cards fly faster, so QA or reproduction scripts should account for pointer motion. ?cite?turn3search0?turn8search5?
- Early builds tied their time step directly to CPU speed, so performance differences were visible from machine to machine. When replicating the effect today, clamp the simulation timestep to avoid frame-rate dependence. ?cite?turn3search1?

## Implementation takeaways for a faithful clone
1. **Capture launch positions.** Snapshot every visible card (foundations, waste, tableau, stock) and store its screen coordinates when the celebration begins; original sol.exe appears to emit from whatever piles are full at trigger time. Use the forced-win shortcut to gather reference data mid-hand.
2. **Emit cards as independent particles.** Community recreations that emulate the Windows look model each card as a sprite with constant downward acceleration, horizontal drift, and damping on bounce (typical values: gravity ˜ -3 px/ms², bounce retention ˜ 0.7, frame step 20 ms). ?cite?turn5view0?
3. **Integrate with a fixed timestep.** Advance velocity and position with a fixed ?t, clamp the number of simulation steps per frame, and apply elasticity when cards hit the window edges or floor.
4. **Layering and trails.** Draw cards in emission order to reproduce the overlapping arcs. Optionally leave faint trails or redraw the background each frame to mimic GDI-era blitting artefacts.
5. **User interaction hooks.** Sample pointer velocity each frame and scale card velocities in response to match the legacy behaviour. Keep the `Alt+Shift+2` hook to let designers trigger (or cancel) the sequence instantly.
6. **Performance safeguards.** Cap the animation duration or stop once all cards settle to avoid runaway simulations—modern hardware renders the sequence much faster than the 16–33 ms frame times the original targeted.

## References for further study
- Video captures of sol.exe on Windows 3.0, 3.1, 95, and XP for frame-by-frame comparison (internal library; record your own if unavailable).
- Wes Cherry interview (b3ta.com) on Solitaire development. ?cite?turn3search1?
- Ars Technica tribute detailing cheat keys and pointer influence. ?cite?turn3search0?
- Community physics reimplementation (peterkhayes/solitaireVictory). ?cite?turn5view0?
