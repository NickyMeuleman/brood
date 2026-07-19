import { useStore } from "@tanstack/react-form";
import { useCallback, useEffect, useState } from "react";
import {
	FieldDescription,
	FieldGroup,
	FieldLegend,
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
		<div className="m-auto mt-6 grid max-w-10/12">
			<div className="space-y-6 lg:col-span-2">
				<form
					onSubmit={(e) => {
						e.preventDefault();
						f.handleSubmit();
					}}
				>
					<FieldGroup>
						<f.AppField name="isin">
							{(field) => (
								<field.TextField
									label="ISIN (International Securities Identification Number)"
									description="Unique instrument identification"
								/>
							)}
						</f.AppField>
						<FieldSet disabled={!isInstrumentEditable}>
							<FieldLegend>Instrument</FieldLegend>
							<FieldDescription>Details for this instrument</FieldDescription>
							<f.AppField name="instrument.name">
								{(field) => (
									<field.TextField label="Name" description="Long name" />
								)}
							</f.AppField>
							<f.AppField name="instrument.issuer">
								{(field) => (
									<field.TextField
										label="Issuer"
										description="Issuing company"
									/>
								)}
							</f.AppField>
							<f.AppField name="instrument.instrument_type">
								{(field) => (
									<field.SelectField
										label="Type"
										options={INSTRUMENT_TYPES.map((v) => ({
											value: v,
											label: v.toLowerCase(),
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
							<f.AppField name="instrument.fsma_registered">
								{(field) => (
									<field.SwitchField
										label="FSMA Registered"
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
								{(field) => <field.SwitchField label="Accumulating" />}
							</f.AppField>
							<f.AppField name="instrument.domicile">
								{(field) => <field.TextField label="Domicile" />}
							</f.AppField>
							<f.AppField name="instrument.subject_to_cgt">
								{(field) => (
									<field.SwitchField
										label="Capital gains tax"
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
						</FieldSet>
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
						<f.AppForm>
							<f.SubmitButton label="Submit" />
						</f.AppForm>
					</FieldGroup>
				</form>
			</div>
		</div>
	);
};

export default ListingAddPage;
