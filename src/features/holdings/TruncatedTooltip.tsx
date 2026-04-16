import {
	type RefObject,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";

interface UseDetectedTruncation<T> {
	ref: RefObject<T | null>;
	isTruncated: boolean;
}

export const useIsTruncated = <
	RefType extends HTMLElement,
>(): UseDetectedTruncation<RefType> => {
	const [isTruncated, setIsTruncated] = useState(false);
	const ref = useRef<RefType>(null);

	const checkTruncation = useCallback(() => {
		const element = ref.current;
		if (!element) return;

		const isWidthTruncated = element.scrollWidth > element.clientWidth;
		const isHeightTruncated = element.scrollHeight > element.clientHeight;

		setIsTruncated(isWidthTruncated || isHeightTruncated);
	}, []);

	useEffect(() => {
		const element = ref.current;
		if (!element) return;

		checkTruncation();

		const resizeObserver = new ResizeObserver(checkTruncation);
		resizeObserver.observe(element);

		return () => {
			resizeObserver.disconnect();
		};
	}, [checkTruncation]);

	return { ref, isTruncated };
};

export const TruncatedTooltip = ({ children }: { children: string }) => {
	const { ref, isTruncated } = useIsTruncated<HTMLParagraphElement>();

	return (
		<Tooltip>
			<TooltipTrigger
				render={
					// css trickery to truncate name but keep lower line as a minimum size
					<p
						ref={ref}
						className="w-0 min-w-full truncate font-medium text-base"
					>
						{children}
					</p>
				}
			/>
			{isTruncated && (
				<TooltipContent side="top">
					<p className="text-sm">{children}</p>
				</TooltipContent>
			)}
		</Tooltip>
	);
};
