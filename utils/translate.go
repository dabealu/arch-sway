package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"strings"
	"time"
)

const translateUrl = "https://translate.google.com/translate_a/single"

func main() {
	text := readInput()
	from, to := detectStringLang(text)

	translated, err := translate(from, to, text)
	if err != nil {
		notify(fmt.Sprintf("failed to translate: %s\n", err))
		return
	}
	notify(translated)
}

func notify(msg string) {
	exec.Command("notify-send", "--app-name=translate", "--urgency=low", "Translation", msg).Run()
}

func readInput() string {
	// try to get input text from stdin...
	fi, err := os.Stdin.Stat()
	if err != nil {
		notify(fmt.Sprintf("failed to get stdin file info: %s\n", err))
		return ""
	}

	if (fi.Mode() & os.ModeCharDevice) == 0 {
		stdinBytes, err := io.ReadAll(os.Stdin)
		if err != nil {
			notify(fmt.Sprintf("failed to read stdin: %s\n", err))
			return ""
		}
		return string(stdinBytes)
	}

	// ...or fallback to args
	if len(os.Args) < 2 {
		notify(fmt.Sprintf("missing input text, usage: %s input text\n", os.Args[0]))
		return ""
	}
	return strings.Join(os.Args[1:], " ")
}

// returns string's lang and lang to which it should be translated
func detectStringLang(s string) (string, string) {
	if len(s) == 0 {
		return "", ""
	}
	sym := s[0]
	// A-Z: 65-90, a-z: 97-122
	if (sym >= 65 && sym <= 90) || (sym >= 97 && sym <= 122) {
		return "en", "ru"
	}
	return "ru", "en"
}

type Sentence struct {
	Trans string `json:"trans"`
	// Orig  string `json:"orig"`
}

type TranslationPayload struct {
	Sentences []Sentence `json:"sentences"`
}

func (tr *TranslationPayload) getSentences() string {
	trans := []string{}
	for _, s := range tr.Sentences {
		trans = append(trans, s.Trans)
	}
	return strings.Join(trans, " ")
}

func buildUrl(from, to, text string) string {
	query := fmt.Sprintf("client=at&dt=t&dj=1&sl=%s&tl=%s&q=%s", from, to, url.QueryEscape(text))
	return fmt.Sprintf("%s?%s", translateUrl, query)
}

func translate(from, to, text string) (string, error) {
	client := &http.Client{
		Timeout: time.Second * 5,
	}

	req, err := http.NewRequest("POST", buildUrl(from, to, text), nil)
	if err != nil {
		return "", err
	}
	req.Header.Add("Content-Type", "application/x-www-form-urlencoded;charset=utf-8")

	resp, err := client.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

	if resp.StatusCode != 200 {
		return "", fmt.Errorf("request failed: %s", resp.Status)
	}

	tr := &TranslationPayload{}
	if err := json.Unmarshal(body, tr); err != nil {
		return "", err
	}

	return tr.getSentences(), nil
}
