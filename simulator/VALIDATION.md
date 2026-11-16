# Simulator validation

검증일: 2026-11-16 (Asia/Seoul). 실제 실행 호스트는 macOS 26.6.2 / arm64 / Apple M1 / 16GiB였습니다. 사용자가 명시한 M1 Pro와 다른 GPU이므로 해당 모델의 성능을 별도로 측정한 결과는 아닙니다. Rust 1.99.0, Cargo 1.99.0, Bevy 0.20.1, serialport 4.10.1.

## 빌드 및 자동 검사

`simulator/`에서 다음 명령이 통과했습니다.

```sh
cargo fmt
cargo fmt --check
cargo check --offline
cargo test --offline
cargo build --offline
cargo test --offline uart_fragment_stale_disconnect_reconnect -- --ignored --nocapture
```

- 일반 테스트: **9 passed**, 0 failed. 기본 실행에서는 환경 의존적인 PTY 테스트 1개를 ignore합니다.
- 별도 PTY 통합 검사: **1 passed**, 0 failed (약 2.2초).
- 파서: CRLF/LF, 잘못된 값, NaN/Inf, 각도 범위, 중복/누락 필드, 손상 UTF-8, 긴 패킷 처리.
- Framing: 256바이트 제한과 다음 줄에서 복구, timeout 사이의 부분 패킷 보존.
- 상태: 정상 패킷의 750ms 유효기간과 포트 닫힘.
- 입력: 영점 보정, 감도/방향 반전/최대각, dead zone, 데이터 소실 후 수평 복귀, 30/120Hz 필터 동등성.
- 물리: 전진, 좌우 선회 방향, 상승/하강, 저속 침하, 지면 충돌 후 정지, 60/120Hz 적분 비교.
- 센서 → 필터 → 비행 통합: 영점 보정 후 기울임으로 선회/상승하고 입력 소실 후 bank/pitch가 수평으로 복귀.
- 실제 UART worker + UNIX PTY: 250ms 간격의 분할 패킷, malformed/oversized 데이터, 200개 연속 패킷의 최신값, 800ms stale 감지, PTY 분리, 경로 재연결 및 새 패킷 수신.

PTY는 Rust `libc` 개발 의존성으로 생성했습니다. Python이나 외부 터미널 프로그램을 사용하지 않습니다. macOS PTY는 물리 baud 설정을 지원하지 않으므로 **해당 private 테스트에서만 baud 0**을 사용합니다. 실제 진입점은 **115200 / 8N1 / no flow control**입니다. 이 검사는 STM32 USB와 실제 baud 동작을 입증하지 않습니다.

macOS 디버그 링커가 `__eh_frame section too large` compact-unwind 경고를 출력했지만 링크와 테스트는 성공했습니다.

## 실제 GPU 실행

```sh
RUST_LOG=info ../target/debug/adxl345_flight_simulator --smoke-test
```

제한된 sandbox 안에서는 GPU 탐색에 실패했습니다. 데스크톱 접근을 허용한 실행에서는 **Apple M1 / Metal** 어댑터를 찾았고 창 생성, 렌더링 및 네 장의 스크린샷 저장을 완료했습니다. 마지막 로그는 `GPU SMOKE PASS: forward flight, bank, climb, recovery; four screenshots`였으며 종료 코드는 0입니다.

| 단계 | 비행 위치 x/y/z (m) | Bank | Pitch |
|---|---|---|---|
| 수평 전진 | 0 / 650 / 1599 | 0° | 0° |
| 오른쪽 선회 | 33 / 648 / 1403 | 30° | 0° |
| 왼쪽 Bank 및 상승 | 88 / 690 / 1231 | -25° | 18° |
| 수평 복귀 | 102 / 700 / 1037 | 0° | 0° |

`validation/flight-0.png`와 `validation/flight-2.png`를 직접 확인했습니다. 비행기의 날개·동체·꼬리·착륙장치, 산과 수면, 구름, 센서와 시뮬레이션 값의 구분, 조작 안내가 보입니다. 모든 단계에서 수치가 유한하고 예상 bank/pitch 및 전진 이동이 확인되며 충돌은 발생하지 않았습니다. 스크린샷은 `validation/flight-0.png` ~ `flight-3.png`이며 git에서는 제외합니다.

이는 **디버그 빌드**의 화면/동작 검사입니다. 스크린샷 당시 프레임 시간은 약 31–40ms였고 셰이더 준비/스크린샷 readback도 포함합니다. 릴리스 빌드 성능 및 지속적인 **60FPS 달성은 미검증**입니다. 앱은 VSync와 120Hz 고정 물리를 사용하며 정상 사용에는 release 실행을 권장합니다.

## CLI 및 기존 프로젝트 보존

- `--help`, `--list-ports` 정상 종료. Bluetooth/debug 장치만 확인됐고 STM32 USB 포트는 없었습니다.
- 루트의 기존 `Cargo.toml`, `Cargo.lock`, `src/`, `assets/`, `README.md`에 대한 `git diff --exit-code`가 통과했습니다. 기존 프로그램과 파일은 변경하지 않았습니다.
- 작업 시작 시 `.ioc`, C/HAL 소스가 없었으므로 펌웨어의 센서 계산, I2C 핀, USART 핀을 실제 소스와 대조하지 못했습니다. 사용자 제공 UART 문법을 유지했습니다.

## 남은 실제 장치/입력 검증

- STM32F439ZI + ADXL345에서 115200 baud 실제 데이터, 각도 축 방향 및 초기 영점 확인.
- 센서를 손으로 기울여 좌우 선회와 기수 상승/하강 확인.
- 물리 USB를 뽑고 다시 꽂아 포트 재탐색과 안전 입력 복귀 확인.
- 실제 키보드 W/S/A/D, 속도 키, Tab 모드 전환, C/I/K, R 및 Esc 입력 확인. GPU 검사는 목표 입력을 프로그램으로 넣으므로 OS 키보드 이벤트 검증은 아닙니다.
- M1 Pro의 release 빌드 프레임 시간 측정.

## 센서 조종 응답 수정 및 실물 UART 추가 확인

후속 점검에서 `/dev/cu.usbmodem11403`이 실제로 연결되어 있었습니다. `--diagnose-serial --port /dev/cu.usbmodem11403`을 데스크톱 권한으로 실행하여 115200 / 8N1에서 6초 동안 **정상 패킷 102개, invalid 0개**를 확인했습니다. 예: `roll=-148.30 pitch=-68.70`부터 `roll=-149.00 pitch=-68.20`까지. 따라서 앞선 기록 중 "실제 baud 수신 미검증"은 이 추가 검사로 해소되었습니다. 손으로 기울였을 때의 축/비행 반응과 물리 USB 재연결은 여전히 사용자 확인이 필요합니다.

기존 초기 보정은 정지 상태 25개를 기다리는 동안 조종을 막아 시작 직후 기울이면 계속 대기할 수 있었습니다. 시작은 첫 정상 패킷으로 영점을 잡아 즉시 조종을 허용하도록 변경했습니다. C를 누르는 수동 보정에만 25개 평균을 적용합니다. Roll이 +/-180° 경계를 넘을 때도 조종 방향이 갑자기 뒤집히지 않도록 각도 차와 평균을 보정했습니다.

변경 후 `cargo fmt`, `cargo check --offline`, `cargo test --offline`, `cargo build --offline` 통과. 일반 테스트는 **11 passed**, PTY 검사 1개 ignored입니다. 초기 입력 즉시 활성화, 수동 평균 보정, Roll 경계 통과 회귀 검사를 포함합니다. 앞선 GPU smoke 결과는 해당 검사 당시 버전에 대한 기록이며 이 후속 수정으로 재실행하지는 않았습니다.
