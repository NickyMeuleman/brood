import { useStore } from "@tanstack/react-form";
import { QueryError } from "@/components/QueryError";
import { FieldGroup } from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import { buyFormOpts, buySchema } from "@/features/trade/shared-form.tsx";
import { useAppForm } from "@/hooks/form";
import { useBrokerFee } from "@/hooks/use-broker-fee";
import { useBrokers } from "@/hooks/use-brokers";
import { useBuy } from "@/hooks/use-buy";
import { useFieldHint } from "@/hooks/use-field-hint";
import { useFx } from "@/hooks/use-fx";
import { useListings } from "@/hooks/use-listings";
import { usePrice } from "@/hooks/use-price";
import { useTobHint } from "@/hooks/use-tob-hint";
import { formatCurrency } from "@/lib/utils";

const BuyPage = () => {
	const buy = useBuy();

	const f = useAppForm({
		...buyFormOpts,
		onSubmit: ({ value }) => {
			const parsed = buySchema.parse(value);
			buy.mutate(parsed);
		},
		formId: "buy_form",
	});

	const { data: brokers = [] } = useBrokers();

	const {
		data: listings = [],
		isLoading: listingsLoading,
		error: listingsError,
	} = useListings();

	const listingId = useStore(f.store, (state) => state.values.listing_id);
	const brokerId = useStore(f.store, (state) => state.values.broker_id);
	const quantity = useStore(f.store, (state) => state.values.quantity);
	const unitPrice = useStore(f.store, (state) => state.values.unit_price);
	const executedAt = useStore(f.store, (state) => state.values.executed_at);
	const brokerFee = useStore(f.store, (state) => state.values.broker_fee);
	const tobFee = useStore(f.store, (state) => state.values.tob_fee);

	const listing = listings.find((l) => l.id === listingId);
	const listingCurrency = listing?.currency_code;

	useFieldHint(f, "unit_price.currency", listing?.currency_code);
	// TODO: all money hints should be sent as Money {currency, amount}, not a plain string
	useFieldHint(f, "broker_fee.currency", brokerFee?.currency);
	useFieldHint(f, "tob_fee.currency", "EUR");

	const isForeignCurrency = Boolean(
		listingCurrency && listingCurrency !== "EUR",
	);

	const broker = brokers.find((b) => b.id === brokerId);

	const {
		data: fxRate,
		error: fxError,
		isLoading: fxLoading,
	} = useFx(executedAt, listingCurrency || "EUR");
	const { data: priceHint } = usePrice(executedAt, listingId);
	const tobHint = useTobHint(
		quantity,
		unitPrice.amount,
		"buy",
		listing?.tob_rate_hint,
		fxRate,
	);

	const { data: brokerFeeHint } = useBrokerFee(
		broker?.broker_type || null,
		quantity,
		unitPrice.amount,
		listing?.instrument_type || "STOCK",
		listing?.exchange_mic || "XAMS",
		fxRate || "1",
	);

	useFieldHint(
		f,
		"unit_price.amount",
		priceHint != null ? Number(priceHint).toFixed(2) : null,
	);
	useFieldHint(
		f,
		"broker_fee.amount",
		brokerFeeHint != null ? Number(brokerFeeHint).toFixed(2) : null,
	);
	useFieldHint(f, "tob_fee.amount", tobHint);

	const base = Number(quantity) * Number(unitPrice.amount);

	let convertedBase: number | null;
	if (!isForeignCurrency) {
		convertedBase = base;
	} else if (fxRate) {
		convertedBase = base * Number(fxRate);
	} else {
		convertedBase = null;
	}

	const isForeignBrokerCurrency =
		brokerFee?.currency && brokerFee?.currency !== "EUR";
	let convertedBroker: number | null;
	if (brokerFee?.currency && !isForeignBrokerCurrency) {
		convertedBroker = Number(brokerFee.amount);
	} else if (fxRate) {
		convertedBroker = Number(brokerFee?.amount || "0") * Number(fxRate);
	} else {
		convertedBroker = null;
	}

	const total =
		(convertedBase ?? 0) +
		(convertedBroker ?? 0) +
		Number(tobFee?.amount || "0");

	console.log({ convertedBase, convertedBroker, tobFee: tobFee?.amount });

	return (
		<div className="m-auto mt-6 grid max-w-10/12 grid-cols-1 gap-12 lg:grid-cols-3">
			<div className="space-y-6 lg:col-span-2">
				{listingsError ? <QueryError message={listingsError.message} /> : null}
				<form
					onSubmit={(e) => {
						e.preventDefault();
						f.handleSubmit();
					}}
				>
					<FieldGroup>
						<f.AppField name="listing_id">
							{(field) => (
								<field.ListingPicker
									label="Listing"
									listings={listings}
									isLoading={listingsLoading}
								/>
							)}
						</f.AppField>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField name="executed_at">
								{(field) => <field.DateTimeField label="Execution time" />}
							</f.AppField>
							<f.AppField name="broker_id">
								{(field) => (
									<field.SelectField
										label="Broker"
										options={brokers.map((b) => ({
											value: b.id,
											label: b.name,
										}))}
										placeholder=""
									/>
								)}
							</f.AppField>
						</div>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField name="quantity">
								{(field) => <field.DecimalField label="Quantity" />}
							</f.AppField>
							<f.AppField name="unit_price">
								{(field) => (
									<field.MoneyField
										label="Unit price"
										currencyDisabled={true}
									/>
								)}
							</f.AppField>
						</div>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField name="broker_fee">
								{(field) => <field.MoneyField label="Broker fee" />}
							</f.AppField>
							<f.AppField name="tob_fee">
								{(field) => (
									<field.MoneyField label="TOB" currencyDisabled={true} />
								)}
							</f.AppField>
						</div>
						<f.AppForm>
							<f.SubmitButton label="Submit" />
						</f.AppForm>
					</FieldGroup>
				</form>
			</div>

			<div className="flex flex-col gap-6">
				<div className="flex flex-col gap-4 rounded-md bg-muted p-6">
					<div className="space-y-4">
						<p className="font-semibold text-lg">Price Summary</p>
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">Base</span>
							<span className="font-semibold text-base">
								{formatCurrency(base, listingCurrency)}
							</span>
						</div>
						{isForeignCurrency && (
							<FxRatePreview
								isLoading={fxLoading}
								error={fxError}
								rate={fxRate}
								convertedValue={convertedBase}
							/>
						)}
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">
								Broker fee
							</span>
							<span className="font-semibold text-base">
								{formatCurrency(
									Number(brokerFee?.amount || "0"),
									brokerFee?.currency,
								)}
							</span>
						</div>
						{isForeignBrokerCurrency && (
							<FxRatePreview
								isLoading={fxLoading}
								error={fxError}
								rate={fxRate}
								convertedValue={convertedBroker}
							/>
						)}
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">TOB</span>
							<span className="font-semibold text-base">
								{formatCurrency(
									Number(tobFee?.amount || "0"),
									tobFee?.currency,
								)}
							</span>
						</div>
					</div>

					<Separator />

					<div className="flex items-center justify-between gap-3">
						<span className="font-semibold text-xl">Total</span>
						<span className="font-semibold text-xl">
							{total === null ? "-" : formatCurrency(total, "EUR")}
						</span>
					</div>
				</div>
			</div>
		</div>
	);
};

function FxRatePreview({
	isLoading,
	error,
	rate,
	convertedValue,
}: {
	isLoading: boolean;
	error: Error | null;
	rate: string | undefined;
	convertedValue: number | null;
}) {
	if (isLoading) {
		return (
			<p className="pl-4 text-muted-foreground text-xs">
				Loading exchange rate…
			</p>
		);
	}

	if (error) {
		return <p className="pl-4 text-destructive text-xs">{error.message}</p>;
	}

	if (!rate || convertedValue === null) {
		return null;
	}

	return (
		<div className="flex items-center justify-between gap-3 pl-4">
			<span className="text-muted-foreground text-sm">Converted to EUR</span>
			<span className="font-medium text-sm">
				{formatCurrency(convertedValue, "EUR")}
			</span>
		</div>
	);
}

export default BuyPage;
