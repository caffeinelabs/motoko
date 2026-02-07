# base/Timer
Timers for one-off or periodic tasks. Applicable as part of the default mechanism.

:::note Timer resolution

The resolution of the timers is in the order of the block rate,
so durations should be chosen well above that. For frequent
canister wake-ups the heartbeat mechanism should be considered.
:::

:::note Overriding system function

The functionality described below is enabled only when the actor does not override it by declaring an explicit `system func timer`.
:::

:::note Upgrade persistence

Timers are _not_ persisted across upgrades. One possible strategy
to re-establish timers after an upgrade is to walk stable variables
in the `post_upgrade` hook and distill necessary timer information
from there.
:::

:::note Security warning

Basing security (e.g. access control) on timers is almost always the wrong choice.
Be sure to inform yourself about state-of-the-art dApp security.
If you _must use_ timers for security controls, be sure to consider reentrancy issues,
and the vanishing of timers on upgrades and reinstalls.
:::

:::note Further information

[Further usage information for timers](https://internetcomputer.org/docs/current/developer-docs/backend/periodic-tasks#timers-library-limitations).
:::

:::note Compilation flag

If `moc` is invoked with `-no-timer`, the importing will fail.
:::

## Type `Duration`
``` motoko no-repl
type Duration = {#seconds : Nat; #nanoseconds : Nat}
```


## Type `TimerId`
``` motoko no-repl
type TimerId = Nat
```


## Function `setTimer`
``` motoko no-repl
func setTimer(d : Duration, job : () -> async ()) : TimerId
```

Installs a one-off timer that upon expiration after given duration `d`
executes the future `job()`.

```motoko no-repl
let now = Time.now();
let thirtyMinutes = 1_000_000_000 * 60 * 30;
func alarmUser() : async () {
  // ...
};
appt.reminder = setTimer(#nanoseconds (Int.abs(appt.when - now - thirtyMinutes)), alarmUser);
```

## Function `recurringTimer`
``` motoko no-repl
func recurringTimer(d : Duration, job : () -> async ()) : TimerId
```

Installs a recurring timer that upon expiration after given duration `d`
executes the future `job()` and reinserts itself for another expiration.

:::info
A duration of 0 will only expire once.
:::

```motoko no-repl
func checkAndWaterPlants() : async () {
  // ...
};
let daily = recurringTimer(#seconds (24 * 60 * 60), checkAndWaterPlants);
```

## Function `cancelTimer`
``` motoko no-repl
func cancelTimer(_ : TimerId) : ()
```

Cancels a still active timer with `(id : TimerId)`. For expired timers
and not recognized `id`s nothing happens.

```motoko no-repl
func deleteAppointment(appointment : Appointment) {
  cancelTimer (appointment.reminder);
  // ...
};
```
