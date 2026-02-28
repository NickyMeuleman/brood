import { useMutation } from "@tanstack/react-query";
import reactLogo from "../assets/react.svg";
import { commands } from "../bindings.ts";
import { useUIStore } from "../stores/ui";
import styles from "./App.module.css";

function App() {
	const name = useUIStore((state) => state.name);
	const setName = useUIStore((state) => state.setName);

	const greetMutation = useMutation({
		mutationFn: (name: string) => commands.greet(name),
	});

	const numbersMutation = useMutation({
		mutationFn: (num: number) => commands.count(num),
	});

	return (
		<main className={styles.container}>
			<h1>Welcome to Tauri + React</h1>

			<div className={styles.row}>
				<a href="https://vite.dev" target="_blank" rel="noopener">
					<img
						src="/vite.svg"
						className={`${styles.logo} ${styles.vite}`}
						alt="Vite logo"
					/>
				</a>
				<a href="https://tauri.app" target="_blank" rel="noopener">
					<img
						src="/tauri.svg"
						className={`${styles.logo} ${styles.tauri}`}
						alt="Tauri logo"
					/>
				</a>
				<a href="https://react.dev" target="_blank" rel="noopener">
					<img
						src={reactLogo}
						className={`${styles.logo} ${styles.react}`}
						alt="React logo"
					/>
				</a>
			</div>
			<p className="text-red-600">
				Click on the Tauri, Vite, and React logos to learn more.
			</p>
			<form
				className={styles.row}
				onSubmit={(e) => {
					e.preventDefault();
					greetMutation.mutate(name);
				}}
			>
				<input
					id="greet-input"
					value={name}
					onChange={(e) => setName(e.currentTarget.value)}
					placeholder="Enter a name..."
				/>
				<button type="submit" disabled={greetMutation.isPending}>
					{greetMutation.isPending ? "Greeting..." : "Greet"}
				</button>
			</form>
			<p>Name global UI state: {name}</p>
			<p>{greetMutation.data}</p>
			<form
				className={styles.row}
				onSubmit={async (e: React.FormEvent<HTMLFormElement>) => {
					e.preventDefault();
					const formData = new FormData(e.currentTarget);
					const num = Number(formData.get("num") || 0);
					numbersMutation.mutate(num);
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
				{(numbersMutation.data || []).map((n) => (
					<li key={n}>{n}</li>
				))}
			</ul>
		</main>
	);
}

export default App;
