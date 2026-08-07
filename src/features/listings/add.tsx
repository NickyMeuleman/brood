import { useStore } from "@tanstack/react-form";
import { Pencil, RefreshCw, Undo2 } from "lucide-react";
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
import { applyHint, setFieldHint, useFieldHint } from "@/hooks/use-field-hint";
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

			addListing.mutate(payload, {
				onSuccess: () => {
					f.reset();
					setIsEditingInstrument(false);
				},
			});
		},
		formId: "listing_add_form",
	});

	const isin = useStore(f.store, (s) => s.values.isin);
	const mic = useStore(f.store, (s) => s.values.listing.mic);
	const ticker = useStore(f.store, (s) => s.values.listing.ticker);
	const instrument_type = useStore(
		f.store,
		(s) => s.values.instrument.instrument_type,
	) as InstrumentType;

	const { data: mics = [] } = useMics();
	const { data: instrumentLookup } = useInstrumentLookup(isin);
	const {
		data: listingSearch,
		isLoading: candidatesLoading,
		error: candidatesError,
	} = useListingCandidates(isin);
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
			applyHint(f, "instrument.name", v.name);
			applyHint(f, "instrument.issuer", v.issuer ?? "");
			applyHint(f, "instrument.instrument_type", v.instrument_type);
			applyHint(f, "instrument.replication", v.replication ?? "");
			applyHint(f, "instrument.fsma_registered", v.fsma_registered);
			applyHint(f, "instrument.accumulating", v.accumulating);
			applyHint(f, "instrument.domicile", v.domicile ?? "");
			applyHint(f, "instrument.subject_to_cgt", v.subject_to_cgt);
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

	// isin based
	useFieldHint(
		f,
		"instrument.instrument_type",
		listingSearch?.instrument_type_hint,
		{ enabled: !isKnownInstrument },
	);
	useFieldHint(f, "instrument.accumulating", listingSearch?.accumulating_hint, {
		enabled: !isKnownInstrument,
	});
	useFieldHint(f, "instrument.domicile", listingSearch?.domicile_hint, {
		enabled: !isKnownInstrument,
	});
	useFieldHint(f, "instrument.issuer", listingSearch?.issuer_hint, {
		enabled: !isKnownInstrument,
	});
	useFieldHint(f, "instrument.name", listingCandidates[0]?.name, {
		enabled: !isKnownInstrument,
	});

	// ticker/mic based - better hints, can replace untouched hints that are already present
	useFieldHint(f, "instrument.name", listingMeta?.name, {
		enabled: !isKnownInstrument,
	});
	useFieldHint(f, "instrument.accumulating", listingMeta?.accumulating_hint, {
		enabled: !isKnownInstrument,
	});
	useFieldHint(f, "listing.currency", listingMeta?.currency);

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
						{candidatesError && (
							<p className="text-destructive text-sm">
								{candidatesError.message}
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
								{instrumentLookup && isEditingInstrument ? (
									<Button
										type="button"
										size="sm"
										variant="secondary"
										onClick={() => {
											setIsEditingInstrument(false);
											setInstrument(instrumentLookup);
										}}
									>
										<Undo2 className="h-3.5 w-3.5" data-icon="inline-start" />
										Reset
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
								{instrument_type === "ETF" ? (
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
								) : null}

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
