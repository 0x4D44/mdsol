# Windows Solitaire Victory Animation Research

## Historical context
- Histories of Microsoft Solitaire emphasize how the end-of-game cascade became part of the franchise's identity and influenced later user-interface celebrations.citeturn18search1

## Original controls and behaviour
- Microsoft retained the `Alt+Shift+2` "force win" shortcut, letting players or testers trigger the cascade on demand even before the game is actually won.citeturn3search0
- Contemporary reporting notes that moving the mouse pointer during the finale speeds the cascade, so faithful recreations should sample pointer motion as part of the update loop.citeturn3search0

## Published pseudocode references
- Community recreations such as the `peterkhayes/solitaireVictory` project publish pseudocode-style hooks: each card (or DOM element) stores velocity, gravity `g`, timestep `dt`, bounce retention, and an `endVelocity` threshold that stops further rebounds. Those scripts provide a reliable template for cloning the original behaviour in modern engines.citeturn15search0

## Derived pseudocode sketch
The following pseudocode summarises the control flow described in the published reimplementation.

```
for each card in launch_order:
    position <- card.start_position
    velocity <- initial_velocity(card)
    while card.visible:
        velocity.y += g * dt
        position += velocity * dt
        if position hits floor:
            velocity.y <- -velocity.y * bounce
            if abs(velocity.y) < endVelocity:
                settle card at floor and mark invisible
        if position.x leaves window bounds:
            wrap card to opposite side (maintain y) and continue
```
citeturn15search0

## Implementation notes
- Factor mouse velocity into the physics step so that quick pointer movement accelerates the cascade, mirroring the legacy shortcut behaviour.citeturn3search0
- Use a fixed timestep (`dt`) and tune `g`/`bounce` values against capture footage to keep the motion recognisably "Windows Solitaire".citeturn15search0
