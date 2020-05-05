#/usr/bin/env python3
import sys
import argparse
import time
from pathlib import Path
import cv2 as cv
import numpy as np

DETECT_METHODS = {
    'orb': cv.ORB_create,
    'kaze': cv.KAZE_create,
    'akaze': cv.AKAZE_create,
    'brisk': cv.BRISK_create,
}
FEATURE_MATCHER = cv.BFMatcher
MIN_MATCHES = 15

VK_ESC = 27

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('-v', '--verbose', default=0, action='count', help='Verbose mode')
    parser.add_argument('-p', '--camera_param_file', type=Path, default=None, help='Camera parameter file for undistortion')
    parser.add_argument('-m', '--method', required=True, choices=list(DETECT_METHODS.keys()), help='Detection method')
    parser.add_argument('-d', '--delay', type=int, default=10, help='Delay between frames in ms')
    parser.add_argument('-s', '--fps', default=False, action='store_true', help='Calculate FPS (require --delay to be 0)')
    parser.add_argument('pattern', type=Path, help='Pattern file for detection')
    parser.add_argument('replace_image', type=Path, help='The image to replace the pattern')
    parser.add_argument('input', type=Path, help='Input video or image file')
    args = parser.parse_args()

    assert not args.fps or args.delay == 0, '--fps require --delay to be 0'

    detector = DETECT_METHODS[args.method]()
    matcher = FEATURE_MATCHER()
    img_pat = cv.imread(str(args.pattern))
    img_replace = cv.imread(str(args.replace_image))

    ar = AR(detector, matcher, img_pat, img_replace, args.verbose)

    video = cv.VideoCapture(str(args.input))
    t = time.time()
    frame_count = 0
    while True:
        ok, frame = video.read()
        if not ok:
            break

        frame_count += 1
        ar.render_frame(frame)

        if args.delay > 0 and cv.waitKey(args.delay) == VK_ESC:
            print('User canceled')
            exit(1)
    t = time.time() - t
    cv.destroyAllWindows()

    if args.fps:
        fps = frame_count / t
        print(f'Processed {frame_count} frames in {t} s. FPS: {fps}')

class AR(object):
    def __init__(self, detector, matcher, img_pat, img_replace, verbose: int):
        self.detector, self.matcher, self.img_pat, self.img_replace, self.verbose = \
            detector, matcher, img_pat, img_replace, verbose
        self.kps_pat, self.descs_pat = self.detector.detectAndCompute(img_pat, None)

        if self.verbose >= 1:
            img_kps = cv.drawKeypoints(self.img_pat, self.kps_pat, self.img_pat, color=(0, 255, 0))
            cv.imshow('Keypoints', img_kps)
            cv.waitKey()
            cv.destroyWindow('Keypoints')

    def _match_features(self, frame):
        kps_frame, descs_frame = self.detector.detectAndCompute(frame, None)
        matches = self.matcher.match(self.descs_pat, descs_frame)
        matches = sorted(matches, key=lambda x: x.distance)

        if self.verbose >= 2:
            print(f'Matches: {len(matches)}')

        if len(matches) < MIN_MATCHES:
            print(f'Not enough matches')
            matches = None
        return kps_frame, descs_frame, matches

    def render_frame(self, frame):
        kps_frame, descs_frame, matches = self._match_features(frame)

        if matches is None:
            return

        if self.verbose >= 2:
            frame_matches = cv.drawMatches(
                self.img_pat,
                self.kps_pat,
                frame,
                kps_frame,
                matches,
                None,
            )
            cv.imshow('Matches', frame_matches)

        src_pts = np.float32([self.kps_pat[m.queryIdx].pt for m in matches]).reshape(-1, 1, 2)
        dst_pts = np.float32([kps_frame[m.trainIdx].pt for m in matches]).reshape(-1, 1, 2)
        M, mask = cv.findHomography(src_pts, dst_pts, cv.RANSAC, 5.0)

        # Draw bounding box.
        if self.verbose >= 1:
            h, w = self.img_pat.shape[:2]
            pts = np.float32([[0, 0], [0, h - 1], [w - 1, h - 1], [w - 1, 0]]).reshape(-1, 1, 2)
            dst = cv.perspectiveTransform(pts, M)
            frame = cv.polylines(frame, [np.int32(dst)], True, 255, 3, cv.LINE_AA)

        h, w = frame.shape[:2]
        warped = np.zeros((h, w, 3), np.uint8)
        warped[:,:,:] = 255
        cv.warpPerspective(self.img_replace, M, (w, h), dst=frame, borderMode=cv.BORDER_TRANSPARENT)

        cv.imshow('Rendered', frame)

if __name__ == '__main__':
    main()
