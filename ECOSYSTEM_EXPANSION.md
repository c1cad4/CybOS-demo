# cybOS ecosystem expansion

This document defines the next cyber-physical layer without making hardware
dependencies part of the default macOS desktop build.

## CybLex

- Embedded librqbit 9.0.1 is the BitTorrent engine.
- The rqbit session runs on a dedicated Tokio worker.
- Session persistence lives under the local cybOS application-support directory.
- CybLex is intended for user-owned, public-domain, open-license, or otherwise
  authorized content.

## Hardware transport

### serialport-rs

serialport is the cross-platform low-level serial transport for USB/RS-232/
RS-485 and device discovery. It is optional and never required by the macOS
desktop runtime.

### embedded-hal

embedded-hal 1.x is the hardware contract layer. cybOS should depend on traits
rather than a concrete microcontroller HAL wherever possible.

### Raspberry Pi

rppal is an optional Raspberry Pi adapter for GPIO/I2C/SPI/PWM/UART. It is
platform-specific and must never become a required dependency of the native
macOS product.

## Mesh

network::mesh defines a transport-neutral packet and route contract.
Meshtastic devices should be connected through a serial transport adapter
rather than copying firmware code or reimplementing the radio firmware.

## VPN

network::vpn defines a WireGuard profile and lifecycle contract. The native
desktop should prefer the operating system's supported WireGuard implementation.
The wireguard-rs userspace implementation is a Linux-oriented adapter and is not
part of the default macOS binary.

## Smart grid

power::modbus contains protocol-safe Modbus RTU framing, CRC16 and energy
policy decisions. Serial transport is injected separately.

Power policy:
NORMAL -> normal UI/camera/polling
CONSERVE -> reduced polling and optional UI power saver
CRITICAL -> minimum telemetry while safety/security systems remain active

No automated write to an inverter is allowed by the protocol layer alone.

## Agentic bus

automation::agent_bus is the allowlisted execution boundary for local LLM
actions. A model produces a typed action request; the bus validates action
type, arguments, caller, deadline, permission and audit metadata.

The bus is deliberately narrower than arbitrary shell execution.

## Bio graph

bio maps physical observations to the existing Cybergraph/SQLite state:
animal -> observation -> event -> relation.

SurrealDB is intentionally not embedded into the desktop core. SQLite +
Cybergraph remains the distributable local state layer.

## Oracle

Switchboard is an optional oracle source for external/on-chain data. It should
feed typed values into the same local event/graph pipeline as weather, market
and sensor data. Oracle input never directly executes a physical actuator.

## Runtime contract

Every new subsystem remains a bounded cell:

INPUT -> VALIDATE -> WORK -> OUTPUT
             |           |
             +-> ERROR <-+
             +-> TIMEOUT

Long-lived transports expose heartbeat and status. UI state never substitutes
an optimistic ONLINE value for a missing heartbeat.
