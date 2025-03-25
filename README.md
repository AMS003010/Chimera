# Chimera - A Fast ⚡ & Powerful JSON Server built with Rust 🦀

## 🔱 Introduction

Chimera is a blazing-fast, configurable JSON server built with Rust and Actix-web. It allows you to serve JSON files as APIs with sorting, pagination, simulated latency, and route-based retrieval. Ideal for prototyping, mock APIs, or rapid development.

## 🚀 Features

- **📂 Serve JSON as an API** – Load any JSON file and serve it as structured API endpoints.
- **📌 Route-based Data Retrieval** – Fetch data by route and ID.
- **📊 Sorting Support** – Sort entries dynamically based on attributes.
- **📑 Pagination Support** – Limit the number of records per request.
- **🐌 Simulated Latency** – Mimic real-world API delays for better testing.
- **⚡ Ultra-Fast Performance** – Leveraging Rust and Actix-web for speed and efficiency.
- **🛠️ Easy Configuration** – Set up ports, file paths, latency, sorting, and pagination via CLI.

## 📦 Installation

### On Windows

On Powershell
```
Invoke-WebRequest -Uri "https://github.com/AMS003010/Chimera/releases/download/v0.1.0/chimera.exe" -OutFile "chimera.exe"
.\chimera.exe --path data.json
```

### On Linux and Mac

### Prerequisites

- Rust (latest stable version)
- Cargo package manager

```sh
git clone https://github.com/your-repo/chimera.git
cd chimera
cargo install --release
chimera --path data.json
```

## 🏗️ Usage

### Start the Server

```sh
./chimera --path data.json
```

### Available Options

| Flag             | Description                                      |
|-----------------|--------------------------------------------------|
| `--path <file>`  | Path to the JSON file (Required)               |
| `--port <port>`  | Specify the server port (Default: 8080)        |
| `--latency <ms>` | Simulated latency in milliseconds (Optional)   |
| `--sort <route> <asc / desc> <attribute>` | Sort route data dynamically |
| `--page <num>`   | Paginate GET responses (Default: 0 - No Limit) |

## 📡 API Endpoints

| Method   | Endpoint        | Description                      |
| -------- | --------------- | -------------------------------- |
| `GET`    | `/ping`         | Health check (`Pong 🏓`)         |
| `GET`    | `/{route}`      | Retrieve all data under a route  |
| `GET`    | `/{route}/{id}` | Retrieve a specific record by ID |
| `DELETE` | `/{route}`      | Delete all records under a route |
| `DELETE` | `/{route}/{id}` | Delete a specific record by ID   |

## 📜 Example JSON File (`data.json`)

```json
{
  "users": [
    { "id": 1, "name": "Alice", "age": 25 },
    { "id": 2, "name": "Bob", "age": 30 }
  ],
  "posts": [
    { "id": 1, "title": "Rust is amazing!" }
  ]
}
```

## 🌟 Why Chimera?

- **Lightweight & Fast** – Runs efficiently with minimal resource usage.
- **Highly Configurable** – Tailor it to your needs with CLI flags.
- **Built for Developers** – Ideal for testing, prototyping, and mock API creation.

## 📜 License

Chimera is licensed under the MIT License.

---

## 👨‍💻 Maintainers
This project is maintained by Abhijith M S (AMS003010).

---

## 📜 License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

## 💡 Contributing
Contributions are welcome! Feel free to open an issue or submit a pull request.

---

## 📩 Contact
For any queries or issues, feel free to reach out via GitHub Issues.

Happy Coding! 🚀

