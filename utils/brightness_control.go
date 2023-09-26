// control brightness of *external* dispaly via i2c-dev/ddcutil
package main

import (
	"bytes"
	"fmt"
	"log"
	"os"
	"os/exec"
	"strconv"
	"strings"
)

const (
	flagErrorMessage = "flag must be specified, one of: increase, decrease, min, max"
	atMinMessage     = "Already at min value"
	atMaxMessage     = "Already at max value"
	displayNumber    = "1"
	valueFile        = "/tmp/brightness.value"
)

func main() {
	if len(os.Args) < 2 {
		log.Fatal(flagErrorMessage)
	}

	loadModule()

	action := os.Args[1]
	switch action {
	case "increase":
		increase()
	case "decrease":
		decrease()
	case "min":
		min()
	case "max":
		max()
	default:
		log.Fatal(flagErrorMessage)
	}
}

// run command and return error, stdout, stderr
func runCmd(com string, args ...string) (error, string, string) {
	cmd := exec.Command(com, args...)

	bufOut, bufErr := bytes.Buffer{}, bytes.Buffer{}
	cmd.Stdout, cmd.Stderr = &bufOut, &bufErr

	err := cmd.Run()
	outStr, errStr := bufOut.String(), bufErr.String()

	return err, outStr, errStr
}

// "grep" through text; returns index when search string starts and if it's been found or not
func grep(txt, search string) (int, bool) {
	txtlen := len(txt)
	searchlen := len(search)

	found := false
	k := 0
	for i := 0; i < txtlen; i++ {
		if txt[i] == search[0] {
			k = i
			for j := 0; j < searchlen; j++ {
				if txt[k] != search[j] {
					break
				}
				k++
			}
			if k-i == searchlen {
				found = true
				break
			}
		}
	}

	return k, found
}

func loadModule() {
	const moduleName = "i2c-dev" // named differently in lsmod

	err, outstr, errstr := runCmd("lsmod")
	if err != nil {
		log.Fatalf("%s:\n%s", err, errstr)
	}

	// grep lsmod to check if module is loaded or no
	_, found := grep(outstr, "i2c_dev")

	// load module
	if !found {
		log.Printf("loading module %s", moduleName)
		if err, _, errstr := runCmd("sudo", "modprobe", moduleName); err != nil {
			log.Fatalf("%s:\n%s", err, errstr)
		}
	}
}

// try to read previously saved value from file, or from cmd
// if both are failing just return max value of 100
func getValue() int {
	// try to read brightness value from file
	valbytes, err := os.ReadFile(valueFile)
	if err == nil {
		if val, err := strconv.Atoi(string(valbytes)); err == nil {
			return val
		}
	}

	// if file doesn't exist yet, try to get brightness value from cmd
	err, outstr, _ := runCmd("sudo", "ddcutil", "--display", displayNumber, "getvcp", "10")
	if err == nil {
		startIdx, startFound := grep(outstr, "Brightness")
		endIdx, endFound := 0, false
		if startFound {
			endIdx, endFound = grep(outstr[startIdx:], "\n")
		}
		if startFound && endFound {
			// sudo ddcutil --display 1 getvcp 10 | grep Bright
			// VCP code 0x10 (Brightness                    ): current value =    67, max value =   100
			words := strings.Split(outstr[startIdx:endIdx], " ")
			valstr := ""
			for j, w := range words {
				if w == "=" {
					valstr = words[j+1]
					break
				}
			}
			val, err := strconv.Atoi(valstr)
			if err == nil {
				return val
			}
		}
	}

	// fallback option, just return static value
	return 100
}

// just write brightness value to a tmp file
func saveValue(val int) {
	if err := os.WriteFile(valueFile, []byte(fmt.Sprintf("%d", val)), 0644); err != nil {
		notification("Failed to save value")
		log.Fatalln(err)
	}
}

func setValue(val int) {
	valstr := fmt.Sprintf("%d", val)
	if err, _, errstr := runCmd("sudo", "ddcutil", "--display", displayNumber, "setvcp", "10", valstr); err != nil {
		notification("Failed to set value")
		log.Fatalf("%s: %s", err, errstr)
	}
	notification(fmt.Sprintf("Set to %d", val))
}

func increase() {
	val := getValue()
	if val == 100 {
		notification(atMaxMessage)
	} else {
		val = calculateNewValue(val, true)
		setValue(val)
	}
	// save in any case - there could be no value saved to file before
	saveValue(val)
}

func decrease() {
	val := getValue()
	if val == 0 {
		notification(atMinMessage)
	} else {
		val = calculateNewValue(val, false)
		setValue(val)
	}
	saveValue(val)
}

func max() {
	if getValue() == 100 {
		notification(atMaxMessage)
	} else {
		setValue(100)
	}
	saveValue(100)
}

func min() {
	if getValue() == 0 {
		notification(atMinMessage)
	} else {
		setValue(0)
	}
	saveValue(0)
}

func calculateNewValue(val int, increase bool) int {
	if increase {
		val += 33
	} else {
		val -= 33
	}

	// keep value in boundaries
	if val < 0 {
		val = 0
	}
	if val > 100 {
		val = 100
	}

	// do some value rounding to avoid changing by
	// few points at values close to min/max
	if val >= 95 {
		val = 100
	}
	if val <= 5 {
		val = 0
	}

	return val
}

func notification(msg string) {
	runCmd("notify-send", "--app-name=brightness-control", "--urgency=low", "Brightness control", msg)
}
