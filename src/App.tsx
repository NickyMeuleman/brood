import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import { commands } from "./bindings";
import "./App.css";

function App() {
	const [greetMsg, setGreetMsg] = useState("");
	const [name, setName] = useState("");
	const [numbers, setNumbers] = useState<Array<number>>([]);
	const [count, setCount] = useState(0);

	useEffect(() => {
		async function getCount() {
			const count = await commands.getCount(1);
			if (count.status === "ok") {
				setCount(count.data.value);
			}
		}
		getCount();
	}, []);

	async function greet() {
		// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
		setGreetMsg(await commands.greet(name));
	}

	return (
		<main className="container">
			<h1>Welcome to Tauri + React</h1>

			<div className="row">
				<a href="https://vite.dev" target="_blank" rel="noopener">
					<img src="/vite.svg" className="logo vite" alt="Vite logo" />
				</a>
				<a href="https://tauri.app" target="_blank" rel="noopener">
					<img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
				</a>
				<a href="https://react.dev" target="_blank" rel="noopener">
					<img src={reactLogo} className="logo react" alt="React logo" />
				</a>
			</div>
			<p>Click on the Tauri, Vite, and React logos to learn more.</p>

			<form
				className="row"
				onSubmit={(e) => {
					e.preventDefault();
					greet();
				}}
			>
				<input
					id="greet-input"
					onChange={(e) => setName(e.currentTarget.value)}
					placeholder="Enter a name..."
				/>
				<button type="submit">Greet</button>
			</form>
			<p>{greetMsg}</p>
			<form
				className="row"
				onSubmit={async (e: React.FormEvent<HTMLFormElement>) => {
					e.preventDefault();
					const formData = new FormData(e.currentTarget);
					const num = formData.get("num");
					const nums = await commands.count(Number(num || 0));
					setNumbers(nums);
				}}
			>
				<input
					name="num"
					id="num-input"
					placeholder="Enter a number..."
					type="number"
				/>
				<button type="submit">count</button>
			</form>
			<ul>
				{numbers.map((n) => (
					<li key={n}>{n}</li>
				))}
			</ul>
			<h2>Count: {count}</h2>
			<button
				type="button"
				onClick={async () => {
					const res = await commands.incrementCount(1);
					if (res.status === "ok") {
						setCount(res.data.value);
					}
				}}
			>
				Increment
			</button>
		</main>
	);
}

export default App;
