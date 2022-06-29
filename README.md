# cctv-exposure

In this work, we present CCTV-Exposure -- the first CCTV-aware solution to evaluate potential privacy exposure to closed-circuit television (CCTV) cameras. 
The objective was to develop a toolset for quantifying human exposure to CCTV cameras from a privacy perspective. 
Our novel approach is trajectory analysis of the individuals, coupled with a database of geo-location mapped CCTV cameras annotated with minimal yet sufficient meta-information. 
For this purpose, CCTV-Exposure model based on a Global Positioning System (GPS) tracking was applied to estimate individual privacy exposure in different scenarios.
The current investigation provides an application example and validation of the modeling approach. 
The methodology and toolset developed and implemented in this work provide time-sequence and location-sequence of the exposure events, thus making possible association of the exposure with the individual activities and cameras, and delivers main statistics on individual's exposure to CCTV cameras with high spatio-temporal resolution.

Originally published in BMSD 2022 12th International Symposium on Business Modeling and Software Design 27-29 June 2022, Fribourg, Switzerland.

## Usage

Python:
- install requirements 
- ` python3 main.py -c <camera  file location> -g <GPX file location> -r <OPTIONAL: selected range-of-vision>`

Rust:
- `cargo run <GPX file location> <camera file location>`

## Citation:

Please cite this work
- `@article{turtiainen2020towards,
  title={CCTVCV: Towards large-scale, automated, accurate detection of CCTV camera objects using computer vision. Applications and implications for privacy, safety, and cybersecurity.(Preprint)},
  author={Turtiainen, Hannu and Costin, Andrei and Lahtinen, Tuomo and Sintonen, Lauri and Hamalainen, Timo},
  journal={arXiv preprint arXiv:2006.03870},
  year={2020}
}`
- `@INPROCEEDINGS{lahtinen2021towards, 
title={Towards CCTV-aware Routing and Navigation for Privacy, Anonymity, and Safety - Feasibility Study in Jyväskylä},  
author={Lahtinen, Tuomo and Sintonen, Lauri and Turtiainen, Hannu and Costin, Andrei}, 
booktitle={2021 28th Conference of Open Innovations Association (FRUCT)},  
year={2021}, 
volume={}, 
number={}, 
pages={252-263},
doi={10.23919/FRUCT50888.2021.9347546}
}`

## License:
<a rel="license" href="http://creativecommons.org/licenses/by-nc/4.0/"><img alt="Creative Commons License" style="border-width:0" src="https://i.creativecommons.org/l/by-nc/4.0/88x31.png" /></a><br />This work is licensed under a <a rel="license" href="http://creativecommons.org/licenses/by-nc/4.0/">Creative Commons Attribution-NonCommercial 4.0 International License</a>.

