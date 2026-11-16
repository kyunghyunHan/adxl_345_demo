# SKYBOUND — Rust / Bevy ADXL345 3D 비행 시뮬레이터

기존 자세 표시 프로그램을 보존하면서 추가한 **독립 Cargo 프로젝트**입니다. 동체·양쪽 날개·꼬리·조종석·착륙장치·회전 프로펠러를 가진 비행기로 산, 호수, 구름, 활주로가 있는 16 × 16 km 지형을 비행합니다. 센서 없이도 키보드로 실행할 수 있습니다.

## 실행

macOS Apple Silicon, Rust **1.97.1 이상** 및 Xcode Command Line Tools가 필요합니다. 현재 환경의 Rust 1.99.0에서 검증합니다. Bevy **0.20.1**, serialport **4.10.1**을 고정했고 `Cargo.lock`을 포함했습니다. Bevy는 Metal을 사용합니다.

```sh
cd /Users/hangyeonghyeon/Desktop/Rust/hardware/adxl_345_demo/simulator
cargo run -- --keyboard
# UART를 열지 않는 키보드 전용 실행
cargo run -- --keyboard --no-serial

# 장치 목록
cargo run -- --list-ports
# ST-LINK를 자동으로 찾고 센서 모드로 시작
cargo run -- --sensor
# 직접 선택
cargo run -- --sensor --port /dev/cu.usbmodemXXXX
# 장착 방향 반전
cargo run -- --sensor --port /dev/cu.usbmodemXXXX --invert-roll --invert-pitch
```

자동 탐색은 `/dev/cu.*` USB 포트 중 STMicroelectronics VID `0483` 장치를 우선합니다. 후보가 여러 개면 임의로 선택하지 않으므로 `--port`로 지정하세요. 연결이 없어도 실행되며 매초 다시 탐색합니다. 다른 시리얼 터미널은 닫아야 합니다. 기본 키보드 모드에서도 UART를 백그라운드에서 수신하므로 Tab으로 바로 전환할 수 있습니다. `--no-serial`은 수신 스레드를 만들지 않습니다.

개발 빌드 최초 컴파일과 첫 GPU 셰이더 준비에는 시간이 걸립니다. 실제 비행에는 `cargo run --release -- --sensor` 또는 `cargo run --release -- --keyboard --no-serial`을 권장합니다. VSync와 120Hz 고정 물리 업데이트를 사용하며 **60FPS는 목표이고 실측 보장은 아닙니다**. `.cargo/config.toml`은 기존 프로젝트의 `../target`을 공유해 엔진 중복 빌드를 줄입니다. 기존 루트 프로젝트와는 별도 패키지/lockfile입니다.

직접 실행 파일은 `../target/debug/adxl345_flight_simulator`입니다. 빌드 시의 절대 assets 경로를 기본으로 사용하므로 현재 작업 디렉터리에 의존하지 않습니다. 다른 컴퓨터로 복사할 때는 assets 폴더도 복사하고 `FLIGHT_ASSET_ROOT=/absolute/path/to/assets`를 지정하세요.

## 조작

| 키 | 기능 |
|---|---|
| W / S | 기수 하강 / 상승 (놓으면 수평 목표) |
| A / D | 왼쪽 / 오른쪽 Roll 및 선회 |
| ↑ / ↓ | 목표 속도 증가 / 감소 (90–360 km/h) |
| Tab | 키보드 / 센서 모드 전환 |
| R | 충돌 여부와 무관하게 초기 위치·속도·방향으로 Reset |
| C | 센서 영점 보정 다시 시작 |
| I / K | 센서 Roll / Pitch 방향 반전 |
| Esc | 종료 |

초기 비행 상태는 고도 650m, 속도 180km/h, 북쪽 방향입니다. 센서 제어에서는 오른쪽 기울임이 양의 Roll, 기수 상승이 양의 Pitch입니다. 실제 센서 장착 방향에 따라 I/K로 반전하세요. ADXL345의 축 방향/펌웨어 각도 부호는 물리 보드에서 확인해야 합니다.

센서 시작 시 **첫 정상 패킷을 영점으로 잡고 즉시 조종을 허용**합니다. 프로그램 시작 때 보드를 원하는 수평 자세로 두세요. 정밀하게 다시 보정하려면 C를 누르고 정지 상태를 유지하세요. 수동 보정은 서로 다른 정상 수신값 25개를 평균내며 평균에서 3° 이상 움직이면 다시 시작합니다. 수동 보정 중에는 센서 조종 목표가 수평이고 HUD에 진행률이 표시됩니다.

센서 수신만 확인하려면 `cargo run -- --diagnose-serial --port /dev/cu.usbmodemXXXX`를 사용하세요. 창 없이 6초 동안 실제 수신값과 패킷 수를 출력합니다. 같은 포트를 사용하는 비행 창이나 시리얼 터미널을 먼저 종료하세요.

## 센서 입력과 비행 모델

- 프로토콜: `ROLL:12.3,PITCH:-5.4\r\n` (LF도 허용), **115200 baud / 8 data bits / no parity / 1 stop bit / no flow control**.
- 별도 OS 스레드가 100ms read timeout으로 수신합니다. 최신값 하나를 담는 mailbox를 렌더 스레드가 `try_lock`으로 가져옵니다. 입력이 몰려도 무한 큐가 생기지 않습니다.
- 부분 패킷은 read timeout 사이에 보존합니다. 최대 256바이트를 넘으면 다음 LF까지 버립니다. 잘못된 UTF-8, 중복/누락/알 수 없는 필드, NaN/Inf, Roll ±180° 또는 Pitch ±90° 밖의 패킷은 무시합니다.
- 장치 오류/분리 시 포트를 닫고 매초 재시도합니다. 정상 패킷이 **750ms** 동안 없으면 STALE 또는 DISCONNECTED가 표시되고 입력은 저역통과필터를 거쳐 수평으로 복귀합니다. 비행과 속도는 계속 유지됩니다. 지형 회피나 자동 착륙 기능은 아닙니다.
- 보정값 차감 → 축 반전 → 연속적인 2° dead zone → 1.5배 감도 → Roll ±55° / Pitch ±25° 제한 → 3Hz 지수 저역통과필터 → 비행 자세 quaternion slerp 순서입니다. 기본값은 `controls.rs`의 `Controls::default()`에 있습니다.
- 좌표는 +X 동쪽 / +Y 고도 / -Z 북쪽입니다. 양의 Roll은 오른쪽 날개 하강, 양의 Pitch는 기수 상승입니다.
- Heading은 `g × tan(bank) / speed`의 협조 선회 근사로 계산합니다. **ADXL345에서 Yaw를 측정하지 않습니다.**
- 엔진은 목표 속도로 가속하고, 중력의 진행 방향 성분이 속도에 영향을 줍니다. 양력은 속도의 제곱에 비례하며 bank의 cos만큼 수직 성분이 줄어듭니다. Pitch가 비행 경로를 바꾸고 낮은 속도에서는 침하합니다. 공기역학/실속의 교육용 단순 근사입니다.
- FixedUpdate 120Hz에서 초 단위 Delta Time으로 적분합니다. 카메라는 위치·회전을 지수 보간하고 지면 아래로 내려가지 않습니다.
- 렌더링과 동일한 삼각형 높이 보간으로 지형 및 수면 충돌을 검사합니다. 충돌하면 멈추고 R을 기다립니다. 비행체 반경 2.5m 근사의 점 검사이며 활주/착륙 모델은 없습니다.
- x/z ±7.6km 또는 고도 4.5km 경계를 넘으면 초기 위치로 자동 Reset하고 HUD 카운터가 증가합니다.

HUD의 **SIMULATED FLIGHT**는 실제 비행체의 Roll/Pitch/Heading, 속도, 해발고도, 지형 또는 수면 기준 AGL, 수직속도입니다. **SENSOR / MEASURED**는 마지막 센서 수신값과 영점이고 **CONTROL / TARGET**은 조종 목표입니다. 연결이 끊겼을 때 마지막 raw 값은 남지만 상태와 입력 목표는 별도로 표시됩니다.

## 기존 프로젝트 확인 결과

이번 작업 시작 시 루트에는 Rust 자세 표시 프로그램(`src/`, `assets/`, `Cargo.toml`)이 있었고, STM32CubeIDE의 `.ioc` / C / HAL 소스는 없었습니다. 따라서 **STM32 펌웨어 구현 및 핀 설정은 실제 소스로 검증할 수 없었습니다**. 기존 프로그램/자산/문서를 변경하지 않았습니다. 펌웨어 변경도 없습니다.

사용자가 제공한 구성은 STM32F439ZI, ADXL345 I2C1 PB8/PB9, USART3 PD8/PD9, ST-LINK VCP입니다. 기존 UART 문법을 그대로 수신하므로 해당 문법을 출력하는 펌웨어에 연결하면 됩니다. 센서가 움직일 때 출력되는 각도의 부호와 USB 재연결은 실제 장치에서 추가 확인해야 합니다.

## 구조 및 자산

```text
simulator/
  Cargo.toml / Cargo.lock
  .cargo/config.toml
  assets/airplane/README.md    외부 모델 없이 생성하는 비행기 안내
  assets/fonts/               Noto Sans와 SIL OFL 라이선스
  src/main.rs                 CLI, Bevy 앱, 시스템 순서
  src/serial.rs               포트 탐색, UART 스레드, framing/parser, 상태
  src/controls.rs             모드, 키보드, 영점, 필터, dead zone
  src/aircraft.rs             복합 Mesh 비행기, 프로펠러
  src/physics.rs              비행, 선회, 양력/중력, 충돌, Reset
  src/terrain.rs              높이 지형, 산, 호수, 구름, 조명
  src/camera.rs               Chase Camera, 거리 안개
  src/hud.rs                  비행/센서/입력 상태 표시
  src/validation.rs           선택 실행 GPU smoke playback
```

비행기와 지형은 모두 Rust 코드에서 생성합니다. 외부 3D 모델이나 네트워크 다운로드가 필요하지 않습니다. 기존 프로젝트에서 복사한 Noto Sans 폰트에는 `assets/fonts/OFL.txt` 라이선스를 포함했습니다.

## 검사

```sh
cargo fmt
cargo check
cargo test
cargo build
# Rust 가상 UART 통합 검사 (로컬 UNIX PTY 접근 필요)
cargo test uart_fragment_stale_disconnect_reconnect -- --ignored --nocapture
# 데스크톱/GPU 필요: 약 20초 시뮬레이션, 첫 셰이더 준비 시간 별도
cargo run -- --smoke-test
```

`--smoke-test`는 실제 UART를 열지 않고 전진 → 오른쪽 선회 → 왼쪽 bank와 상승 → 수평 복귀를 구동합니다. 유한한 상태값, 전진 거리, bank/pitch 반응, 충돌 여부를 확인하고 `validation/flight-0.png`부터 네 장을 저장한 뒤 자동 종료합니다. 실제 키보드 이벤트나 물리 STM32의 검증을 대신하지 않습니다. 이 작업에서 얻은 결과와 미검증 항목은 `VALIDATION.md`를 참고하세요.

일반 테스트 11개가 통과했고, 별도 Rust PTY 통합 검사와 Apple M1 / Metal의 GPU smoke 실행도 앞선 검사에서 통과했습니다. 후속 점검에서는 실제 `/dev/cu.usbmodem11403`에서 115200 baud 정상 패킷 102개(오류 0개)를 수신했습니다. 실제 손 기울임과 비행 방향의 대응, 물리 USB 재연결, 릴리스 60FPS는 추가 확인이 필요합니다.

버전 참고: [Bevy 0.20.1](https://docs.rs/bevy/0.20.1/bevy/), [Bevy 최소 Rust 버전](https://docs.rs/crate/bevy/0.20.1/source/Cargo.toml), [serialport 4.10.1](https://docs.rs/serialport/4.10.1/serialport/).
