import re

from streamlink.plugin import Plugin, pluginmatcher
from streamlink.stream.http import HTTPStream


@pluginmatcher(re.compile(r"https://streamlink-contract\.invalid/(?P<version>8\.(?:0|4))"))
class Contract(Plugin):
    def _get_streams(self):
        streams = {
            "1080p60_h264": HTTPStream(
                self.session, "https://streamlink-contract.invalid/video-h264"
            ),
            "720p_h264": HTTPStream(
                self.session, "https://streamlink-contract.invalid/video-720"
            ),
        }
        if self.match.group("version") == "8.4":
            streams.update(
                {
                    "1440p60_hevc": HTTPStream(
                        self.session, "https://streamlink-contract.invalid/video-hevc"
                    ),
                    "2160p60_av1": HTTPStream(
                        self.session, "https://streamlink-contract.invalid/video-av1"
                    ),
                    "future_ultra": HTTPStream(
                        self.session, "https://streamlink-contract.invalid/video-future"
                    ),
                }
            )
        return streams


__plugin__ = Contract
