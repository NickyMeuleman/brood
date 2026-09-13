import { useQuery } from "@tanstack/react-query";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { type BrokerType, commands, type InstrumentType } from "../bindings";

export function useBrokerFee(
	broker: BrokerType | null,
  listing_currency: string,
	quantity: string,
	unitPrice: string,
	instrumentType: InstrumentType,
	mic: string,
	fxRate: string,
) {
	return useQuery({
		queryKey: queryKeys.brokerFeeFor(
			broker,
      listing_currency,
			quantity,
			unitPrice,
			instrumentType,
			mic,
			fxRate,
		),
		queryFn: async () => {
			const res = await commands.brokerFeeHint(
				broker,
        listing_currency,
				quantity,
				unitPrice,
				instrumentType,
				mic,
				fxRate,
			);
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}

			return res.data;
		},
	});
}
