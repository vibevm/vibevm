"""The one way this package declines to act."""


class Refusal(Exception):
    """Every reason a program in this package declines to move or report, as a list.

    Collected rather than raised at the first sight of trouble: an operator told
    about one broken path fixes it and meets the next one, and a migration that
    reveals its objections one per run is a migration nobody finishes.

    It is an exception rather than a return value because every caller's correct
    response is the same — stop, print, write nothing — and a return value that
    means "stop" is a return value somebody eventually forgets to check.
    """

    def __init__(self, headline, details=()):
        super().__init__(headline)
        self.headline = headline
        self.details = list(details)


def report_refusal(refusal, out):
    """The one wording, so a refusal reads the same on every surface."""
    print(f"REFUSING — {refusal.headline}", file=out)
    for detail in refusal.details:
        if detail:
            print(f"  {detail}", file=out)
