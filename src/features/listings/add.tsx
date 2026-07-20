import { useStore } from "@tanstack/react-form";
import { Pencil } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	FieldGroup,
	FieldLegend,
	FieldSeparator,
	FieldSet,
} from "@/components/ui/field";
import {
	INSTRUMENT_TYPES,
	listingAddFormOpts,
	listingAddSchema,
	REPLICATIONS,
} from "@/features/listings/shared-form";
import { useAppForm } from "@/hooks/form";
import { useInstrumentLookup } from "@/hooks/use-instrument-lookup";
import { useMics } from "@/hooks/use-mics";

const ListingAddPage = () => {
	const f = useAppForm({
		...listingAddFormOpts,
		onSubmit: async ({ value }) => {
			console.log(value);
			const parsed = listingAddSchema.parse(value);
			console.log(parsed);
		},
		formId: "listing_add_form",
	});

	const { data: mics = [] } = useMics();
	const isin = useStore(f.store, (s) => s.values.isin);
	const { data: instrumentLookup } = useInstrumentLookup(isin);
	const isKnownInstrument = !!instrumentLookup;
	const [isEditingInstrument, setIsEditingInstrument] = useState(false);
	const isInstrumentEditable = !isKnownInstrument || isEditingInstrument;

	const setInstrument = useCallback(
		async (v: typeof instrumentLookup) => {
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
		if (!instrumentLookup) return;
		setInstrument(instrumentLookup);
		setIsEditingInstrument(false);
	}, [instrumentLookup, setInstrument]);

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
											options={INSTRUMENT_TYPES.map((v) => ({
												value: v,
												label: v,
											}))}
											placeholder="Type of instrument"
										/>
									)}
								</f.AppField>
								<f.AppField name="instrument.replication">
									{(field) => (
										<field.SelectField
											label="Replication"
											options={REPLICATIONS.map((v) => ({
												value: v,
												label: v.toLowerCase(),
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
							<FieldLegend className="mb-6 font-bold">
								Listing Details
							</FieldLegend>
							<FieldGroup className="grid gap-6 md:grid-cols-3">
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

export default ListingAddPage;
