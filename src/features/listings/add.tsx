import { useStore } from "@tanstack/react-form";
import { Pencil, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import type {
	AddListingInput,
	InstrumentType,
	ListingCandidate,
	Replication,
} from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldGroup,
	FieldLabel,
	FieldLegend,
	FieldSeparator,
	FieldSet,
	FieldTitle,
} from "@/components/ui/field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
	INSTRUMENT_TYPES,
	listingAddFormOpts,
	listingAddSchema,
	REPLICATIONS,
} from "@/features/listings/shared-form";
import { useAppForm } from "@/hooks/form";
import { useAddListing } from "@/hooks/use-add-listing";
import { useInstrumentLookup } from "@/hooks/use-instrument-lookup";
import { useListingCandidates } from "@/hooks/use-listing-candidates";
import { useListingMeta } from "@/hooks/use-listing-meta";
import { useMics } from "@/hooks/use-mics";
import { MIC_LABEL } from "@/lib/utils";

const ListingAddPage = () => {
	const addListing = useAddListing();

	const f = useAppForm({
		...listingAddFormOpts,
		onSubmit: async ({ value }) => {
			const parsed = listingAddSchema.parse(value);
			let payload: AddListingInput;

			if (!instrumentLookup) {
				payload = {
					kind: "NewInstrument",
					instrument: {
						isin: parsed.isin,
						...parsed.instrument,
					},
					listing: parsed.listing,
				};
			} else if (!isEditingInstrument) {
				payload = {
					kind: "ExistingInstrument",
					instrument_id: instrumentLookup.id,
					listing: parsed.listing,
				};
			} else {
				payload = {
					kind: "UpdateInstrument",
					instrument_id: instrumentLookup.id,
					instrument: {
						isin: parsed.isin,
						...parsed.instrument,
					},
					listing: parsed.listing,
				};
			}

			addListing.mutate(payload);
			f.reset();
			setIsEditingInstrument(false);
		},
		formId: "listing_add_form",
	});

	const isin = useStore(f.store, (s) => s.values.isin);
	const mic = useStore(f.store, (s) => s.values.listing.mic);
	const ticker = useStore(f.store, (s) => s.values.listing.ticker);

	const { data: mics = [] } = useMics();
	const { data: instrumentLookup } = useInstrumentLookup(isin);
	const { data: listingSearch, isLoading: candidatesLoading } =
		useListingCandidates(isin);
	const listingCandidates = listingSearch?.candidates ?? [];
	const isKnownInstrument = !!instrumentLookup;
	const { data: listingMeta, isLoading: listingMetaLoading } = useListingMeta(
		mic,
		ticker,
	);

	const [isEditingInstrument, setIsEditingInstrument] = useState(false);
	const isInstrumentEditable = !isKnownInstrument || isEditingInstrument;

	// existing instrument
	const setInstrument = useCallback(
		(v: typeof instrumentLookup) => {
			if (!v) return;
			f.setFieldValue("instrument.name", v.name);
			f.setFieldValue("instrument.issuer", v.issuer ?? "");
			f.setFieldValue("instrument.instrument_type", v.instrument_type);
			f.setFieldValue("instrument.replication", v.replication ?? "");
			f.setFieldValue("instrument.fsma_registered", v.fsma_registered);
			f.setFieldValue("instrument.accumulating", v.accumulating);
			f.setFieldValue("instrument.domicile", v.domicile ?? "");
			f.setFieldValue("instrument.subject_to_cgt", v.subject_to_cgt);
		},
		[f],
	);
	useEffect(() => {
		if (!instrumentLookup) {
			return;
		}
		setInstrument(instrumentLookup);
		setIsEditingInstrument(false);
	}, [instrumentLookup, setInstrument]);

	// instrument_type and accumulating
	const instrumentTypePristine = useStore(
		f.store,
		(s) => s.fieldMeta["instrument.instrument_type"]?.isPristine,
	);
	const accumulatingPristine = useStore(
		f.store,
		(s) => s.fieldMeta["instrument.accumulating"]?.isPristine,
	);
	useEffect(() => {
		// a real stored fact always outranks a hint
		if (isKnownInstrument || !listingSearch) return;

		if (instrumentTypePristine && listingSearch.instrument_type_hint) {
			f.setFieldValue(
				"instrument.instrument_type",
				listingSearch.instrument_type_hint,
			);
		}
		if (accumulatingPristine && listingSearch.accumulating_hint != null) {
			f.setFieldValue(
				"instrument.accumulating",
				listingSearch.accumulating_hint,
			);
		}
	}, [
		listingSearch,
		instrumentTypePristine,
		accumulatingPristine,
		isKnownInstrument,
		f,
	]);

	// FIGI name (basic, only based on isin)
	const namePristine = useStore(
		f.store,
		(s) => s.fieldMeta["instrument.name"]?.isPristine,
	);

	useEffect(() => {
		if (isKnownInstrument || !namePristine) return;
		const name = listingCandidates?.[0]?.name;
		if (name) {
			f.setFieldValue("instrument.name", name);
			f.setFieldMeta("instrument.name", (prev) => ({
				...prev,
				isTouched: false,
				isDirty: false,
				isPristine: true,
			}));
		}
	}, [listingCandidates, namePristine, isKnownInstrument, f]);

	// Yahoo name (better, based on ticker + mic)
	useEffect(() => {
		if (isKnownInstrument || !namePristine || !listingMeta?.name) return;
		f.setFieldValue("instrument.name", listingMeta.name);
	}, [listingMeta, namePristine, isKnownInstrument, f]);

	// accumulating — second chance against the fuller name, same pristine gate
	useEffect(() => {
		if (
			isKnownInstrument ||
			!accumulatingPristine ||
			listingMeta?.accumulating_hint == null
		) {
			return;
		}
		f.setFieldValue("instrument.accumulating", listingMeta.accumulating_hint);
	}, [listingMeta, accumulatingPristine, isKnownInstrument, f]);

	// currency
	const currencyPristine = useStore(
		f.store,
		(s) => s.fieldMeta["listing.currency"]?.isPristine,
	);
	useEffect(() => {
		if (!currencyPristine || !listingMeta?.currency) return;
		f.setFieldValue("listing.currency", listingMeta.currency);
	}, [listingMeta, currencyPristine, f]);

	return (
		<div className="m-auto mt-8 max-w-4xl p-4">
			<div className="space-y-6">
				<form
					onSubmit={(e) => {
						e.preventDefault();
						f.handleSubmit();
					}}
				>
					<FieldGroup>
						<FieldGroup>
							<f.AppField name="isin">
								{(field) => (
									<field.TextField
										label="ISIN"
										description="International Securities Identification Number"
									/>
								)}
							</f.AppField>
						</FieldGroup>

						{candidatesLoading && (
							<p className="flex items-center gap-1.5 text-muted-foreground text-sm">
								<RefreshCw className="h-3.5 w-3.5 animate-spin" />
								Looking up listings for this ISIN…
							</p>
						)}

						{listingCandidates.length > 0 ? (
							<ListingCandidatesGrid
								candidates={listingCandidates}
								selected={{ mic, ticker }}
								onSelect={(item) => {
									f.setFieldValue("listing.mic", item.mic);
									f.setFieldValue("listing.ticker", item.ticker);
								}}
							/>
						) : null}

						<FieldSeparator />

						<FieldSet
							disabled={!isInstrumentEditable}
							aria-disabled={!isInstrumentEditable}
						>
							<FieldLegend className="mb-6 flex items-center justify-between font-bold">
								Instrument Details
								{instrumentLookup && !isInstrumentEditable ? (
									<Button
										type="button"
										size="sm"
										onClick={() => setIsEditingInstrument(true)}
									>
										<Pencil className="h-3.5 w-3.5" data-icon="inline-start" />
										Enable Editing
									</Button>
								) : null}
							</FieldLegend>
							<FieldGroup className="grid gap-6 md:grid-cols-2">
								<div className="col-span-full">
									<f.AppField name="instrument.name">
										{(field) => <field.TextField label="Name" />}
									</f.AppField>
								</div>

								<f.AppField name="instrument.issuer">
									{(field) => <field.TextField label="Issuer" />}
								</f.AppField>
								<f.AppField name="instrument.domicile">
									{(field) => <field.TextField label="Domicile" />}
								</f.AppField>
								<f.AppField name="instrument.instrument_type">
									{(field) => (
										<field.SelectField
											label="Type"
											options={Object.entries(INSTRUMENT_TYPES).map(
												([k, v]) => ({
													value: k as InstrumentType,
													label: v,
												}),
											)}
											placeholder="Type of instrument"
										/>
									)}
								</f.AppField>
								<f.AppField name="instrument.replication">
									{(field) => (
										<field.SelectField
											label="Replication"
											options={Object.entries(REPLICATIONS).map(([k, v]) => ({
												value: k as Replication,
												label: v,
											}))}
											placeholder="Replication method"
										/>
									)}
								</f.AppField>

								<FieldGroup className="col-span-full">
									<f.AppField name="instrument.fsma_registered">
										{(field) => (
											<field.SwitchField
												label="FSMA Registered"
												disabled={!isInstrumentEditable}
												description={
													<>
														Look up registration status on the{" "}
														<a
															href="https://www.fsma.be/nl/data-portal"
															target="_blank"
															rel="noopener"
														>
															FSMA data portal
														</a>
													</>
												}
											/>
										)}
									</f.AppField>
									<f.AppField name="instrument.accumulating">
										{(field) => (
											<field.SwitchField
												label="Accumulating"
												disabled={!isInstrumentEditable}
											/>
										)}
									</f.AppField>
									<f.AppField name="instrument.subject_to_cgt">
										{(field) => (
											<field.SwitchField
												label="Capital gains tax"
												disabled={!isInstrumentEditable}
												description={
													<>
														Some products are exempt.{" "}
														<a
															href="https://fin.belgium.be/nl/particulieren/belastingaangifte/inkomsten/meerwaardebelasting#wat-zijn-financiele-activa"
															target="_blank"
															rel="noopener"
														>
															source: FOD Financien
														</a>
													</>
												}
											/>
										)}
									</f.AppField>
								</FieldGroup>
							</FieldGroup>
						</FieldSet>

						<FieldSeparator />

						<FieldSet>
							<FieldLegend className="mb-6 flex items-center justify-between font-bold">
								Listing Details
								{listingMetaLoading && (
									<p className="flex items-center gap-1.5 text-muted-foreground text-sm">
										<RefreshCw className="h-3.5 w-3.5 animate-spin" />
										Loading listing info...
									</p>
								)}
							</FieldLegend>
							<FieldGroup className="grid items-start gap-6 md:grid-cols-3">
								<f.AppField name="listing.mic">
									{(field) => (
										<field.ExchangePicker label="Exchange" exchanges={mics} />
									)}
								</f.AppField>
								<f.AppField name="listing.ticker">
									{(field) => <field.TextField label="Ticker" />}
								</f.AppField>
								<f.AppField name="listing.currency">
									{(field) => <field.TextField label="Currency" />}
								</f.AppField>
							</FieldGroup>
						</FieldSet>

						<FieldGroup>
							<f.AppForm>
								<f.SubmitButton label="Submit" />
							</f.AppForm>
						</FieldGroup>
					</FieldGroup>
				</form>
			</div>
		</div>
	);
};

function ListingCandidatesGrid({
	candidates,
	selected,
	onSelect,
}: {
	candidates: ListingCandidate[];
	selected: { mic: string; ticker: string };
	onSelect: (candidate: ListingCandidate) => void;
}) {
	if (!candidates || candidates.length === 0) return null;

	const selectedIdx = candidates.findIndex(
		(c: ListingCandidate) =>
			c.mic === selected.mic && c.ticker === selected.ticker,
	);

	return (
		<FieldSet className="w-full">
			<FieldLegend variant="label">Suggestions</FieldLegend>
			<FieldDescription>This ISIN trades under these listings</FieldDescription>
			<RadioGroup
				value={selectedIdx === -1 ? null : selectedIdx}
				onValueChange={(idx) => {
					const chosenListing = candidates[idx];
					if (chosenListing) onSelect(chosenListing);
				}}
				className="grid grid-cols-1 md:grid-cols-3"
			>
				{candidates.map((candidate, i) => {
					const key = `${candidate.mic}-${candidate.ticker}`;
					return (
						<FieldLabel key={key} htmlFor={key}>
							<Field orientation="horizontal">
								<FieldContent>
									<FieldTitle>{candidate.ticker}</FieldTitle>
									<FieldDescription>
										{MIC_LABEL[candidate.mic]
											? `${candidate.mic} - ${MIC_LABEL[candidate.mic]}`
											: `${candidate.mic}`}
									</FieldDescription>
								</FieldContent>
								<RadioGroupItem value={i} id={key} />
							</Field>
						</FieldLabel>
					);
				})}
			</RadioGroup>
		</FieldSet>
	);
}
export default ListingAddPage;
