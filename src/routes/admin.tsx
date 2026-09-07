import { createFileRoute } from "@tanstack/react-router";
import { AlertCircle, CheckCircle2, RefreshCw } from "lucide-react";
import { useMemo, useRef } from "react";
import type {
	BrokerType,
	FXSyncOutcome,
	ImportRowOutcome,
	PriceSyncOutcome,
} from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	Item,
	ItemContent,
	ItemDescription,
	ItemTitle,
} from "@/components/ui/item";
import { useAppForm } from "@/hooks/form";
import { useAddBroker } from "@/hooks/use-add-broker";
import { useBrokers } from "@/hooks/use-brokers";
import {
	useForceUpdateAllFx,
	useForceUpdateAllPrices,
	useForceUpdateOneCurrencyFx,
	useForceUpdateOneListingPrices,
} from "@/hooks/use-force-sync";
import { useImportBuyCSV } from "@/hooks/use-import";
import { useListings } from "@/hooks/use-listings";
import { cn } from "@/lib/utils";
export const Route = createFileRoute("/admin")({
	component: RouteComponent,
	staticData: { title: "Admin" },
});

const BROKER_TYPE_MAP: Record<BrokerType, null> = {
	BOLERO: null,
	REBEL: null,
	MEDIRECT: null,
	SAXO: null,
};

function RouteComponent() {
	const allPrices = useForceUpdateAllPrices();
	const allFx = useForceUpdateAllFx();
	const fileInputRef = useRef<HTMLInputElement>(null);
	const importMutation = useImportBuyCSV();
	const brokers = useBrokers();
	const addBroker = useAddBroker();

	const form = useAppForm({
		defaultValues: {
			name: "",
			broker_type: "" as BrokerType | "",
		},
		onSubmit: async ({ value }) => {
			await addBroker.mutateAsync({
				name: value.name.trim(),
				broker_type: value.broker_type ? value.broker_type : null,
			});
			form.reset();
		},
	});

	const handleButtonClick = () => {
		// Programmatically click the hidden file input
		fileInputRef.current?.click();
	};

	const handleFileChange = async (
		event: React.ChangeEvent<HTMLInputElement>,
	) => {
		const file = event.target.files?.[0];
		if (!file) return;

		try {
			// Read the file directly into a string using standard modern Web APIs
			const csvText = await file.text();

			// Fire off the string to your Tauri Rust command
			importMutation.mutate(csvText);
		} catch (error) {
			console.error("Failed to read file", error);
		} finally {
			// Clear the input value so the same file can be uploaded back-to-back if needed
			if (event.target) event.target.value = "";
		}
	};

	const { data: listings = [] } = useListings();

	const currencies = useMemo(() => {
		if (!listings) return [];
		const set = new Set(
			listings
				.filter((h) => h.currency_code !== "EUR")
				.map((h) => h.currency_code),
		);

		return [...set];
	}, [listings]);

	return (
		<div className="space-y-6 p-4">
			<section className="space-y-3">
				<h3 className="font-medium text-muted-foreground text-sm uppercase tracking-widest">
					Price history
				</h3>
				<div className="flex items-center gap-3">
					<Button
						variant="outline"
						disabled={allPrices.isPending}
						onClick={() => allPrices.mutate()}
					>
						Force update all prices
					</Button>
					<MutationStatus mutation={allPrices} />
				</div>
				<OutcomeList outcomes={allPrices.data} />
				{listings.length > 0 && (
					<div className="mt-2 flex flex-col gap-2 border-l-2 pt-2 pl-4">
						{listings.map((l) => (
							<ListingPriceRow key={l.id} listing_id={l.id} {...l} />
						))}
					</div>
				)}
			</section>

			<section className="space-y-3">
				<h3 className="font-medium text-muted-foreground text-sm uppercase tracking-widest">
					FX rates
				</h3>
				<div className="flex items-center gap-3">
					<Button
						variant="outline"
						disabled={allFx.isPending}
						onClick={() => allFx.mutate()}
					>
						Force update all FX rates
					</Button>
					<MutationStatus mutation={allFx} />
				</div>
				<OutcomeList outcomes={allFx.data} />
				{currencies.length > 0 && (
					<div className="mt-2 flex flex-col gap-2 border-l-2 pt-2 pl-4">
						{currencies.map((c) => (
							<CurrencyRow key={c} currency={c} />
						))}
					</div>
				)}
			</section>

			<section className="space-y-3">
				<h3 className="font-medium text-muted-foreground text-sm uppercase tracking-widest">
					Import
				</h3>
				<div className="flex items-center gap-3">
					<input
						type="file"
						ref={fileInputRef}
						onChange={handleFileChange}
						accept=".csv"
						className="hidden"
					/>
					<Button
						variant="outline"
						disabled={allPrices.isPending}
						onClick={handleButtonClick}
					>
						Import buy trade csv
					</Button>
					<MutationStatus mutation={importMutation} />
				</div>
				<ImportOutcomeList outcomes={importMutation.data} />
			</section>

			<section className="space-y-3">
				<h3 className="font-medium text-muted-foreground text-sm uppercase tracking-widest">
					Brokers
				</h3>

				<form
					onSubmit={(e) => {
						e.preventDefault();
						e.stopPropagation();
						form.handleSubmit();
					}}
					className="grid max-w-xl gap-3 rounded-lg border bg-card p-4"
				>
					<form.AppField
						name="name"
						children={(field) => (
							<field.TextField
								label="Name"
								description="eg. Rebel at Belfius"
							/>
						)}
					/>

					<form.AppField
						name="broker_type"
						children={(field) => (
							<field.SelectField
								label="Type"
								options={Object.entries(BROKER_TYPE_MAP).map(([k, v]) => ({
									value: k,
									label: k,
								}))}
								placeholder=""
							/>
						)}
					/>

					<form.AppForm>
						<form.SubmitButton label="Submit" />
					</form.AppForm>
				</form>
				{brokers.isError && <p>Brokers error</p>}
				{brokers.isLoading && <p>Brokers loading</p>}
				{brokers.data && (
					<div className="grid w-fit grid-cols-3 gap-3">
						{brokers.data.map((b) => {
							return (
								<Item key={b.id} variant="outline">
									<ItemContent>
										<ItemTitle>{b.name}</ItemTitle>
										<ItemDescription>Type: {b.broker_type}</ItemDescription>
									</ItemContent>
								</Item>
							);
						})}
					</div>
				)}
			</section>
		</div>
	);
}

function Outcome({
	outcome,
	label,
}: {
	outcome: FXSyncOutcome | PriceSyncOutcome;
	label?: string;
}) {
	const isOk = outcome.status === "success";
	return (
		<li className={cn("flex items-center gap-2", !isOk && "text-destructive")}>
			{isOk ? (
				<CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-emerald-500" />
			) : (
				<AlertCircle className="h-3.5 w-3.5 shrink-0" />
			)}
			{label && <span className="font-mono">{label}</span>}
			{isOk ? (
				<span className="text-muted-foreground">
					{outcome.added} rows upserted
				</span>
			) : (
				<span>{outcome.message}</span>
			)}
		</li>
	);
}

function OutcomeList({
	outcomes,
}: {
	outcomes: FXSyncOutcome[] | PriceSyncOutcome[] | undefined;
}) {
	if (!outcomes?.length) return null;
	return (
		<ul className="space-y-1 text-sm">
			{outcomes.map((o) => {
				const label = "ticker" in o ? o.ticker : o.currency;
				return <Outcome key={label} label={label} outcome={o} />;
			})}
		</ul>
	);
}

function MutationStatus({
	mutation,
}: {
	mutation: {
		isPending: boolean;
		isError: boolean;
		isSuccess: boolean;
		error: Error | null;
	};
}) {
	if (mutation.isPending) {
		return <span className="text-muted-foreground text-sm">Running…</span>;
	}
	if (mutation.isError) {
		return (
			<span className="text-destructive text-sm">
				{mutation.error?.message ?? "Failed"}
			</span>
		);
	}
	if (mutation.isSuccess) {
		return <span className="text-emerald-600 text-sm">Done</span>;
	}
	return null;
}

function ListingPriceRow({
	listing_id,
	exchange_mic,
	ticker,
}: {
	listing_id: number;
	exchange_mic: string;
	ticker: string;
}) {
	const sync = useForceUpdateOneListingPrices();
	return (
		<div className="flex items-center gap-3">
			<Button
				variant="outline"
				size="sm"
				disabled={sync.isPending}
				onClick={() => sync.mutate({ listing_id, exchange_mic, ticker })}
			>
				{sync.isPending && <RefreshCw className="animate-spin" />}
				Sync {ticker} ({exchange_mic})
			</Button>
			{sync.isSuccess && <Outcome outcome={sync.data} />}
			{sync.isError && !sync.data && (
				<Outcome
					outcome={{ status: "error", ticker, message: sync.error.message }}
				/>
			)}
		</div>
	);
}

function CurrencyRow({ currency }: { currency: string }) {
	const sync = useForceUpdateOneCurrencyFx();
	return (
		<div className="flex items-center gap-3">
			<Button
				variant="outline"
				size="sm"
				disabled={sync.isPending}
				onClick={() => sync.mutate(currency)}
			>
				{sync.isPending && <RefreshCw className="animate-spin" />}
				Sync {currency} rates
			</Button>
			{sync.isSuccess && <Outcome outcome={sync.data} />}
			{sync.isError && (
				<Outcome
					outcome={{ status: "error", currency, message: sync.error.message }}
				/>
			)}
		</div>
	);
}

function ImportOutcomeList({
	outcomes,
}: {
	outcomes: ImportRowOutcome[] | undefined;
}) {
	if (!outcomes?.length) return null;
	return (
		<ul className="space-y-1 text-sm">
			{outcomes.map((o) => (
				<li
					key={o.row}
					className={cn(
						"flex items-center gap-2",
						o.status === "error" && "text-destructive",
					)}
				>
					{o.status === "success" ? (
						<CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-emerald-500" />
					) : (
						<AlertCircle className="h-3.5 w-3.5 shrink-0" />
					)}
					<span className="font-mono">Row {o.row}</span>
					{o.status === "error" && <span>{o.message}</span>}
				</li>
			))}
		</ul>
	);
}
