import reactLogo from "./assets/react.svg";
import { commands } from "./bindings";
import "./App.css";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useUIStore } from "./stores/ui";

function App() {
	const queryClient = useQueryClient();
	const name = useUIStore((state) => state.name);
	const setName = useUIStore((state) => state.setName);

	const countQuery = useQuery({
		queryKey: ["count", 1],
		queryFn: async () => {
			const res = await commands.getCount(1);
			if (res.status === "error") throw new Error("Failed to fetch count");
			return res.data.value;
		},
	});

	const greetMutation = useMutation({
		mutationFn: (name: string) => commands.greet(name),
	});

	const numbersMutation = useMutation({
		mutationFn: (num: number) => commands.count(num),
	});

	const incrementMutation = useMutation({
		mutationFn: (val: number) => commands.incrementCount(val),
		onSuccess: (data, id) => {
			queryClient.invalidateQueries({ queryKey: ["count", id] });
		},
	});

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
				className="row"
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
			<h2>Count: {countQuery.isLoading ? "loading" : countQuery.data}</h2>
			<button type="button" onClick={() => incrementMutation.mutate(1)}>
				Increment
			</button>
		</main>
	);
}

export default App;
